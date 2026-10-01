use super::helpers::*;
use super::{path_ref, response, validate, BridgeState};
use crate::contract::*;
use tauri::{Runtime, State, Window};

#[tauri::command]
pub async fn picker_select<R: Runtime>(
    request: Req<UniversePickerSelectRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<UniversePickerSelectResult> {
    let request_id = validate(&request, false)?;
    let dialog = rfd::FileDialog::new().set_parent(&window);
    crate::debug::info("PICKER", format!("Opening directory picker for kind: {:?}", request.payload.kind));
    let selected_opt =
        match tauri::async_runtime::spawn_blocking(move || dialog.pick_folder()).await {
            Ok(opt) => opt,
            Err(source) => {
                crate::debug::error("PICKER", format!("Picker worker failed: {source}"));
                return Err(error(
                    IpcErrorCode::Internal,
                    format!("picker worker failed: {source}"),
                ));
            }
        };
    let selected = match selected_opt {
        Some(folder) => {
            crate::debug::success("PICKER", format!("Selected directory: {folder:?}"));
            folder
        }
        None => {
            crate::debug::warn("PICKER", "Directory selection was cancelled by user.");
            return Err(error(
                IpcErrorCode::InvalidArgument,
                "directory selection was cancelled",
            ));
        }
    };
    let selected =
        state.replace_picker_directory(window.label(), request.payload.kind, &selected)?;
    Ok(response(
        request_id,
        UniversePickerSelectResult {
            kind: request.payload.kind,
            path: path_ref(selected),
        },
    ))
}
