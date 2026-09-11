use super::{CanonicalRoots, NativeProgress, outside_roots, random_token};
use crate::CancellationToken;
use rustix::fd::{AsFd, OwnedFd};
use rustix::fs::{
    AtFlags, Dir, FileType, Mode, OFlags, RenameFlags, fchmod, fstat, mkdirat, open, openat, renameat_with, statat,
    unlinkat,
};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

#[cfg(test)]
thread_local! {
    static DURABILITY_EVENTS: std::cell::RefCell<Vec<&'static str>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(super) fn record_durability_event_for_test(event: &'static str) {
    DURABILITY_EVENTS.with(|events| events.borrow_mut().push(event));
}

#[cfg(test)]
pub(super) fn take_durability_events_for_test() -> Vec<&'static str> {
    DURABILITY_EVENTS.with(|events| std::mem::take(&mut *events.borrow_mut()))
}

#[derive(Clone, Copy)]
struct ObjectIdentity {
    dev: u64,
    ino: u64,
}

const OPEN_DIR: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const OPEN_FILE: OFlags = OFlags::RDONLY.union(OFlags::NOFOLLOW).union(OFlags::CLOEXEC);

pub(super) fn validate_target_directory(path: &Path, roots: &CanonicalRoots) -> io::Result<PathBuf> {
    let root = roots
        .paths()
        .iter()
        .find(|root| root.as_path() == path)
        .ok_or_else(outside_roots)?;
    let fd = open_directory(root)?;
    ensure_directory(&fd)?;
    Ok(root.clone())
}

pub(super) fn validate_recorded_path(
    path: &Path,
    operation_token: &str,
    roots: &CanonicalRoots,
) -> io::Result<PathBuf> {
    let parent = path.parent().ok_or_else(outside_roots)?;
    let name = normal_file_name(path)?;
    let root = roots
        .paths()
        .iter()
        .find(|root| root.as_path() == parent)
        .ok_or_else(outside_roots)?;
    let root_fd = open_directory(root)?;
    let identity = verify_operation_marker(&root_fd, operation_token)?;
    if let Err(error) = verify_entry_identity(&root_fd, name, identity) {
        if error.kind() != io::ErrorKind::NotFound {
            return Err(error);
        }
        let quarantine = quarantine_name(operation_token);
        match verify_entry_identity(&root_fd, &quarantine, identity) {
            Ok(_) => {}
            Err(quarantine_error) if quarantine_error.kind() == io::ErrorKind::NotFound => {
                resume_pending_install(&root_fd, name, operation_token, identity)?;
            }
            Err(quarantine_error) => return Err(quarantine_error),
        }
    }
    Ok(path.to_path_buf())
}

pub(super) fn preflight(
    artifact: &Path,
    destination: &Path,
    artifact_roots: &CanonicalRoots,
    user_roots: &CanonicalRoots,
    cancellation: &CancellationToken,
) -> io::Result<()> {
    cancellation.check()?;
    let source = open_artifact(artifact, artifact_roots)?;
    count_opened(&source.fd, source.kind, cancellation)?;
    let (target_fd, destination_name) = open_destination_parent(destination, user_roots)?;
    require_absent(&target_fd, destination_name)?;
    Ok(())
}

pub(super) fn count_artifact(
    artifact: &Path,
    roots: &CanonicalRoots,
    cancellation: &CancellationToken,
) -> io::Result<u64> {
    let source = open_artifact(artifact, roots)?;
    count_opened(&source.fd, source.kind, cancellation)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn install_artifact(
    artifact: &Path,
    destination: &Path,
    operation_token: &str,
    artifact_roots: &CanonicalRoots,
    user_roots: &CanonicalRoots,
    cancellation: &CancellationToken,
    total: u64,
    progress: &mut impl FnMut(NativeProgress),
    before_publish: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    cancellation.check()?;
    let source = open_artifact(artifact, artifact_roots)?;
    let (target_fd, destination_name) = open_destination_parent(destination, user_roots)?;
    require_absent(&target_fd, destination_name)?;
    let stage_name = stage_name(operation_token);
    mkdirat(&target_fd, &stage_name, Mode::RWXU).map_err(errno)?;
    let mut record_committed = false;
    let mut marker_created = false;
    let mut completed = 0;
    let result = (|| {
        let stage_fd = openat(&target_fd, &stage_name, OPEN_DIR, Mode::empty()).map_err(errno)?;
        copy_root(
            &source,
            &stage_fd,
            OsStr::new("payload"),
            artifact,
            cancellation,
            total,
            &mut completed,
            progress,
        )?;
        sync_fd(&stage_fd, "install-stage")?;
        cancellation.check()?;
        let identity = identity_of(&stat_no_follow(&stage_fd, OsStr::new("payload"))?);
        create_operation_marker(&target_fd, operation_token, identity)?;
        marker_created = true;
        sync_fd(&target_fd, "install-target-before-record")?;
        // From this point persistence may have committed even if it reports an
        // error (for example, a parent-directory fsync failure). Keep the
        // marker and stage so durable state can be reloaded and resumed.
        record_committed = true;
        before_publish()?;
        renameat_with(
            &stage_fd,
            "payload",
            &target_fd,
            destination_name,
            RenameFlags::NOREPLACE,
        )
        .map_err(errno)?;
        let _ = sync_fd(&target_fd, "install-publish-parent");
        // The NOREPLACE rename is the commit point. A best-effort cleanup error
        // after it must not report a failed install or orphan its persisted record.
        let _ = unlinkat(&target_fd, &stage_name, AtFlags::REMOVEDIR);
        Ok(())
    })();
    if result.is_err() && !record_committed {
        let _ = remove_entry(&target_fd, &stage_name, &CancellationToken::new());
        if marker_created {
            let _ = unlinkat(&target_fd, operation_marker_name(operation_token), AtFlags::empty());
        }
    }
    result
}

pub(super) fn uninstall_artifact(
    installed_path: &Path,
    operation_token: &str,
    roots: &CanonicalRoots,
    cancellation: &CancellationToken,
) -> io::Result<()> {
    cancellation.check()?;
    let (root_fd, installed_name) = open_destination_parent(installed_path, roots)?;
    let identity = verify_operation_marker(&root_fd, operation_token)?;
    let quarantine = quarantine_name(operation_token);

    match stat_no_follow(&root_fd, installed_name) {
        Ok(_) => {
            verify_entry_identity(&root_fd, installed_name, identity)?;
            renameat_with(&root_fd, installed_name, &root_fd, &quarantine, RenameFlags::NOREPLACE).map_err(errno)?;
            verify_entry_identity(&root_fd, &quarantine, identity)?;
            sync_fd(&root_fd, "uninstall-quarantine-parent")?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            verify_entry_identity(&root_fd, &quarantine, identity)?;
        }
        Err(error) => return Err(error),
    }

    if let Err(error) = cancellation.check() {
        let current_identity = verify_operation_marker(&root_fd, operation_token)?;
        if current_identity.dev != identity.dev || current_identity.ino != identity.ino {
            return Err(identity_changed("operation marker identity changed"));
        }
        verify_entry_identity(&root_fd, &quarantine, identity)?;
        return match renameat_with(&root_fd, &quarantine, &root_fd, installed_name, RenameFlags::NOREPLACE) {
            Ok(()) => {
                sync_fd(&root_fd, "uninstall-restore-parent")?;
                Err(error)
            }
            Err(restore) => Err(io::Error::other(format!(
                "uninstall cancelled and restore failed; quarantine retained for retry: {}",
                errno(restore)
            ))),
        };
    }
    // Re-read both the marker and quarantine immediately before recursive
    // deletion. A reused quarantine name must never authorize another inode.
    let current_identity = verify_operation_marker(&root_fd, operation_token)?;
    if current_identity.dev != identity.dev || current_identity.ino != identity.ino {
        return Err(identity_changed("operation marker identity changed"));
    }
    verify_entry_identity(&root_fd, &quarantine, identity)?;
    remove_entry(&root_fd, &quarantine, cancellation)?;
    sync_fd(&root_fd, "uninstall-delete-parent")?;
    // Quarantine deletion is the uninstall commit; marker cleanup is best effort.
    let _ = unlinkat(&root_fd, operation_marker_name(operation_token), AtFlags::empty());
    let _ = sync_fd(&root_fd, "uninstall-marker-parent");
    Ok(())
}

#[cfg(test)]
pub(super) fn quarantine_then_cancel_for_test(
    installed_path: &Path,
    operation_token: &str,
    roots: &CanonicalRoots,
) -> io::Result<()> {
    let (root_fd, installed_name) = open_destination_parent(installed_path, roots)?;
    let quarantine = quarantine_name(operation_token);
    renameat_with(&root_fd, installed_name, &root_fd, &quarantine, RenameFlags::NOREPLACE).map_err(errno)?;
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    if let Err(error) = cancellation.check() {
        renameat_with(&root_fd, &quarantine, &root_fd, installed_name, RenameFlags::NOREPLACE).map_err(errno)?;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn quarantine_cancel_with_restore_collision_for_test(
    installed_path: &Path,
    operation_token: &str,
    roots: &CanonicalRoots,
) -> io::Result<()> {
    let (root_fd, installed_name) = open_destination_parent(installed_path, roots)?;
    let quarantine = quarantine_name(operation_token);
    renameat_with(&root_fd, installed_name, &root_fd, &quarantine, RenameFlags::NOREPLACE).map_err(errno)?;
    let collision = openat(
        &root_fd,
        installed_name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(errno)?;
    drop(collision);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    if let Err(error) = cancellation.check() {
        return match renameat_with(&root_fd, &quarantine, &root_fd, installed_name, RenameFlags::NOREPLACE) {
            Ok(()) => Err(error),
            Err(restore) => Err(io::Error::other(format!(
                "uninstall cancelled and restore failed; quarantine retained for retry: {}",
                errno(restore)
            ))),
        };
    }
    Ok(())
}

pub(super) fn reconcile_pending_uninstall(
    installed_path: &Path,
    operation_token: &str,
    roots: &CanonicalRoots,
    cancellation: &CancellationToken,
) -> io::Result<bool> {
    let (root, installed_name) = open_destination_parent(installed_path, roots)?;
    let quarantine = quarantine_name(operation_token);
    let installed = stat_no_follow(&root, installed_name);
    let quarantined = stat_no_follow(&root, &quarantine);

    match (installed, quarantined) {
        (Ok(_), Err(error)) if error.kind() == io::ErrorKind::NotFound => {
            let identity = verify_operation_marker(&root, operation_token)?;
            verify_entry_identity(&root, installed_name, identity)?;
            Ok(true)
        }
        (Err(installed_error), Ok(_)) if installed_error.kind() == io::ErrorKind::NotFound => {
            finish_quarantined_uninstall(&root, &quarantine, operation_token, cancellation)?;
            Ok(false)
        }
        (Ok(_), Ok(_)) => {
            let identity = verify_operation_marker(&root, operation_token)?;
            verify_entry_identity(&root, &quarantine, identity)?;
            if verify_entry_identity(&root, installed_name, identity).is_ok() {
                return Err(identity_changed("installed path and quarantine conflict"));
            }
            finish_quarantined_uninstall(&root, &quarantine, operation_token, cancellation)?;
            Ok(false)
        }
        (Err(installed_error), Err(quarantine_error))
            if installed_error.kind() == io::ErrorKind::NotFound
                && quarantine_error.kind() == io::ErrorKind::NotFound =>
        {
            match verify_operation_marker(&root, operation_token) {
                Ok(_) => {
                    unlinkat(&root, operation_marker_name(operation_token), AtFlags::empty()).map_err(errno)?;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            Ok(false)
        }
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}

pub(super) fn abandon_pending_install(
    installed_path: &Path,
    operation_token: &str,
    roots: &CanonicalRoots,
) -> io::Result<()> {
    let (root, _) = open_destination_parent(installed_path, roots)?;
    let stage = stage_name(operation_token);
    let stage_fd = openat(&root, &stage, OPEN_DIR, Mode::empty()).map_err(errno)?;
    let identity = verify_operation_marker(&root, operation_token)?;
    verify_entry_identity(&stage_fd, OsStr::new("payload"), identity)?;
    remove_entry(&root, &stage, &CancellationToken::new())?;
    unlinkat(&root, operation_marker_name(operation_token), AtFlags::empty()).map_err(errno)
}

fn finish_quarantined_uninstall(
    root: &OwnedFd,
    quarantine: &OsStr,
    operation_token: &str,
    cancellation: &CancellationToken,
) -> io::Result<()> {
    let identity = verify_operation_marker(root, operation_token)?;
    verify_entry_identity(root, quarantine, identity)?;
    remove_entry(root, quarantine, cancellation)?;
    sync_fd(root, "uninstall-delete-parent")?;
    unlinkat(root, operation_marker_name(operation_token), AtFlags::empty()).map_err(errno)?;
    sync_fd(root, "uninstall-marker-parent")
}

fn resume_pending_install(
    root: &OwnedFd,
    destination: &OsStr,
    operation_token: &str,
    identity: ObjectIdentity,
) -> io::Result<()> {
    let stage = stage_name(operation_token);
    let stage_fd = openat(root, &stage, OPEN_DIR, Mode::empty()).map_err(errno)?;
    verify_entry_identity(&stage_fd, OsStr::new("payload"), identity)?;
    renameat_with(&stage_fd, "payload", root, destination, RenameFlags::NOREPLACE).map_err(errno)?;
    verify_entry_identity(root, destination, identity)?;
    let _ = unlinkat(root, &stage, AtFlags::REMOVEDIR);
    Ok(())
}

fn stage_name(operation_token: &str) -> OsString {
    OsString::from(format!(".universal-converter-stage-{operation_token}"))
}

fn quarantine_name(operation_token: &str) -> OsString {
    OsString::from(format!(".universal-converter-uninstall-{operation_token}"))
}

fn operation_marker_name(operation_token: &str) -> OsString {
    OsString::from(format!(".universal-converter-owner-{operation_token}"))
}

fn create_operation_marker(root: &OwnedFd, operation_token: &str, identity: ObjectIdentity) -> io::Result<()> {
    let name = operation_marker_name(operation_token);
    let marker = openat(
        root,
        &name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(errno)?;
    let mut marker = File::from(marker);
    if let Err(error) = marker
        .write_all(format!("{operation_token}\n{}\n{}\n", identity.dev, identity.ino).as_bytes())
        .and_then(|()| marker.sync_all())
    {
        drop(marker);
        let _ = unlinkat(root, &name, AtFlags::empty());
        return Err(error);
    }
    Ok(())
}

fn verify_operation_marker(root: &OwnedFd, operation_token: &str) -> io::Result<ObjectIdentity> {
    let marker = openat(root, operation_marker_name(operation_token), OPEN_FILE, Mode::empty()).map_err(errno)?;
    if FileType::from_raw_mode(fstat(&marker).map_err(errno)?.st_mode) != FileType::RegularFile {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "invalid operation marker",
        ));
    }
    let mut content = String::new();
    File::from(marker).read_to_string(&mut content)?;
    let mut lines = content.lines();
    let token = lines.next();
    let dev = lines.next().and_then(|value| value.parse::<u64>().ok());
    let ino = lines.next().and_then(|value| value.parse::<u64>().ok());
    if token != Some(operation_token) || lines.next().is_some() {
        return Err(identity_changed("operation marker mismatch"));
    }
    match (dev, ino) {
        (Some(dev), Some(ino)) => Ok(ObjectIdentity { dev, ino }),
        _ => Err(identity_changed("operation marker has invalid identity")),
    }
}

fn identity_of(stat: &rustix::fs::Stat) -> ObjectIdentity {
    ObjectIdentity {
        dev: stat.st_dev,
        ino: stat.st_ino,
    }
}

fn verify_entry_identity<Fd: AsFd>(parent: Fd, name: &OsStr, expected: ObjectIdentity) -> io::Result<()> {
    let actual = stat_no_follow(parent, name)?;
    if actual.st_dev == expected.dev && actual.st_ino == expected.ino {
        Ok(())
    } else {
        Err(identity_changed("published inode identity changed"))
    }
}

fn identity_changed(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

struct OpenedSource {
    fd: OwnedFd,
    kind: FileType,
    mode: Mode,
}

fn open_artifact(path: &Path, roots: &CanonicalRoots) -> io::Result<OpenedSource> {
    let root = roots
        .paths()
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .ok_or_else(outside_roots)?;

    if path == root {
        if let Ok(fd) = open_directory(root) {
            let stat = fstat(&fd).map_err(errno)?;
            return Ok(OpenedSource {
                fd,
                kind: FileType::Directory,
                mode: Mode::from_raw_mode(stat.st_mode),
            });
        }
        let parent = root.parent().ok_or_else(outside_roots)?;
        let parent_fd = open_directory(parent)?;
        return open_child(&parent_fd, normal_file_name(root)?);
    }

    let mut current = open_directory(root)?;
    let relative = path.strip_prefix(root).map_err(|_| outside_roots())?;
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else {
            return Err(outside_roots());
        };
        if components.peek().is_none() {
            return open_child(&current, name);
        }
        stat_expected(&current, name, FileType::Directory)?;
        current = openat(&current, name, OPEN_DIR, Mode::empty()).map_err(errno)?;
        ensure_directory(&current)?;
    }
    Err(outside_roots())
}

fn open_child(parent: &OwnedFd, name: &OsStr) -> io::Result<OpenedSource> {
    let stat = stat_no_follow(parent, name)?;
    let kind = FileType::from_raw_mode(stat.st_mode);
    let flags = match kind {
        FileType::Directory => OPEN_DIR,
        FileType::RegularFile => OPEN_FILE,
        FileType::Symlink => return Err(outside_roots()),
        _ => return Err(special_file()),
    };
    let fd = openat(parent, name, flags, Mode::empty()).map_err(errno)?;
    let opened = fstat(&fd).map_err(errno)?;
    let opened_kind = FileType::from_raw_mode(opened.st_mode);
    if opened_kind != kind || opened.st_dev != stat.st_dev || opened.st_ino != stat.st_ino {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "artifact changed while opening",
        ));
    }
    Ok(OpenedSource {
        fd,
        kind,
        mode: Mode::from_raw_mode(opened.st_mode),
    })
}

fn count_opened(fd: &OwnedFd, kind: FileType, cancellation: &CancellationToken) -> io::Result<u64> {
    cancellation.check()?;
    match kind {
        FileType::RegularFile => Ok(1),
        FileType::Directory => {
            let mut total = 0_u64;
            for name in directory_names(fd)? {
                let child = open_child(fd, &name)?;
                total = total
                    .checked_add(count_opened(&child.fd, child.kind, cancellation)?)
                    .ok_or_else(|| io::Error::other("artifact file count overflow"))?;
            }
            Ok(total)
        }
        _ => Err(special_file()),
    }
}

#[allow(clippy::too_many_arguments)]
fn copy_root(
    source: &OpenedSource,
    target_parent: &OwnedFd,
    target_name: &OsStr,
    display_path: &Path,
    cancellation: &CancellationToken,
    total: u64,
    completed: &mut u64,
    progress: &mut impl FnMut(NativeProgress),
) -> io::Result<()> {
    match source.kind {
        FileType::RegularFile => copy_file(
            &source.fd,
            source.mode,
            target_parent,
            target_name,
            display_path,
            cancellation,
            total,
            completed,
            progress,
        ),
        FileType::Directory => {
            mkdirat(target_parent, target_name, Mode::RWXU).map_err(errno)?;
            let target = openat(target_parent, target_name, OPEN_DIR, Mode::empty()).map_err(errno)?;
            for name in directory_names(&source.fd)? {
                cancellation.check()?;
                let child = open_child(&source.fd, &name)?;
                copy_root(
                    &child,
                    &target,
                    &name,
                    &display_path.join(&name),
                    cancellation,
                    total,
                    completed,
                    progress,
                )?;
            }
            fchmod(&target, safe_mode(source.mode)).map_err(errno)?;
            sync_fd(&target, "install-directory")
        }
        _ => Err(special_file()),
    }
}

#[allow(clippy::too_many_arguments)]
fn copy_file(
    source: &OwnedFd,
    source_mode: Mode,
    target_parent: &OwnedFd,
    target_name: &OsStr,
    display_path: &Path,
    cancellation: &CancellationToken,
    total: u64,
    completed: &mut u64,
    progress: &mut impl FnMut(NativeProgress),
) -> io::Result<()> {
    let target = openat(
        target_parent,
        target_name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(errno)?;
    let mut input = File::from(source.as_fd().try_clone_to_owned()?);
    let mut output = File::from(target);
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        cancellation.check()?;
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
    }
    fchmod(&output, safe_mode(source_mode)).map_err(errno)?;
    sync_fd(&output, "install-file")?;
    *completed += 1;
    progress(NativeProgress {
        completed: *completed,
        total,
        path: Some(display_path.to_path_buf()),
    });
    Ok(())
}

fn remove_entry(parent: &OwnedFd, name: &OsStr, cancellation: &CancellationToken) -> io::Result<()> {
    remove_entry_with(parent, name, cancellation, || Ok(()))
}

fn remove_entry_with(
    parent: &OwnedFd,
    name: &OsStr,
    cancellation: &CancellationToken,
    before_non_directory_rename: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    cancellation.check()?;
    let before = stat_no_follow(parent, name)?;
    if FileType::from_raw_mode(before.st_mode) != FileType::Directory {
        before_non_directory_rename()?;
        return tombstone_then_unlink(parent, name, identity_of(&before));
    }
    let directory = openat(parent, name, OPEN_DIR, Mode::empty()).map_err(errno)?;
    let opened = fstat(&directory).map_err(errno)?;
    if opened.st_dev != before.st_dev || opened.st_ino != before.st_ino {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "directory changed while opening",
        ));
    }
    for child in directory_names(&directory)? {
        remove_entry(&directory, &child, cancellation)?;
    }
    sync_fd(&directory, "delete-directory")?;
    let after = stat_no_follow(parent, name)?;
    if after.st_dev != opened.st_dev || after.st_ino != opened.st_ino {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "directory changed before unlink",
        ));
    }
    unlinkat(parent, name, AtFlags::REMOVEDIR).map_err(errno)
}

fn tombstone_then_unlink(parent: &OwnedFd, name: &OsStr, expected: ObjectIdentity) -> io::Result<()> {
    let tombstone = loop {
        let candidate = OsString::from(format!(".universal-converter-delete-{}", random_token()?));
        match renameat_with(parent, name, parent, &candidate, RenameFlags::NOREPLACE) {
            Ok(()) => break candidate,
            Err(error) if error == rustix::io::Errno::EXIST => continue,
            Err(error) => return Err(errno(error)),
        }
    };
    verify_entry_identity(parent, &tombstone, expected)?;
    unlinkat(parent, &tombstone, AtFlags::empty()).map_err(errno)
}

fn sync_fd<Fd: AsFd>(fd: Fd, _event: &'static str) -> io::Result<()> {
    File::from(fd.as_fd().try_clone_to_owned()?).sync_all()?;
    #[cfg(test)]
    record_durability_event_for_test(_event);
    Ok(())
}

#[cfg(test)]
pub(super) fn swap_regular_before_tombstone_for_test(path: &Path, roots: &CanonicalRoots) -> io::Result<()> {
    let (parent, name) = open_destination_parent(path, roots)?;
    remove_entry_with(&parent, name, &CancellationToken::new(), || {
        renameat_with(
            &parent,
            name,
            &parent,
            ".universal-converter-test-original",
            RenameFlags::NOREPLACE,
        )
        .map_err(errno)?;
        let replacement = openat(
            &parent,
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(errno)?;
        File::from(replacement).write_all(b"attacker replacement")
    })
}

fn directory_names(fd: &OwnedFd) -> io::Result<Vec<OsString>> {
    let mut directory = Dir::read_from(fd).map_err(errno)?;
    let mut names = Vec::new();
    while let Some(entry) = directory.read() {
        let entry = entry.map_err(errno)?;
        let bytes = entry.file_name().to_bytes();
        if bytes != b"." && bytes != b".." {
            names.push(OsStr::from_bytes(bytes).to_os_string());
        }
    }
    Ok(names)
}

fn open_destination_parent<'a>(destination: &'a Path, roots: &CanonicalRoots) -> io::Result<(OwnedFd, &'a OsStr)> {
    let parent = destination.parent().ok_or_else(outside_roots)?;
    let root = roots
        .paths()
        .iter()
        .find(|root| root.as_path() == parent)
        .ok_or_else(outside_roots)?;
    Ok((open_directory(root)?, normal_file_name(destination)?))
}

fn open_directory(path: &Path) -> io::Result<OwnedFd> {
    if !path.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "path must be absolute"));
    }
    let mut current = open(Path::new("/"), OPEN_DIR, Mode::empty()).map_err(errno)?;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                stat_expected(&current, name, FileType::Directory)?;
                current = openat(&current, name, OPEN_DIR, Mode::empty()).map_err(errno)?;
                ensure_directory(&current)?;
            }
            _ => return Err(outside_roots()),
        }
    }
    Ok(current)
}

fn ensure_directory(fd: &OwnedFd) -> io::Result<()> {
    if FileType::from_raw_mode(fstat(fd).map_err(errno)?.st_mode) == FileType::Directory {
        Ok(())
    } else {
        Err(outside_roots())
    }
}

fn stat_no_follow<Fd: AsFd>(parent: Fd, name: &OsStr) -> io::Result<rustix::fs::Stat> {
    statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(errno)
}

fn stat_expected(parent: &OwnedFd, name: &OsStr, expected: FileType) -> io::Result<()> {
    let actual = FileType::from_raw_mode(stat_no_follow(parent, name)?.st_mode);
    if actual == expected {
        Ok(())
    } else if actual == FileType::Symlink {
        Err(outside_roots())
    } else {
        Err(special_file())
    }
}

fn require_absent(parent: &OwnedFd, name: &OsStr) -> io::Result<()> {
    match stat_no_follow(parent, name) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "installation target already exists",
        )),
    }
}

fn normal_file_name(path: &Path) -> io::Result<&OsStr> {
    path.file_name()
        .filter(|name| !name.as_bytes().is_empty() && *name != OsStr::new(".") && *name != OsStr::new(".."))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path must have a normal file name"))
}

fn safe_mode(mode: Mode) -> Mode {
    mode & (Mode::RWXU | Mode::RWXG | Mode::RWXO)
}

fn special_file() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "artifact must contain only regular files and directories",
    )
}

fn errno(error: rustix::io::Errno) -> io::Error {
    io::Error::from_raw_os_error(error.raw_os_error())
}
