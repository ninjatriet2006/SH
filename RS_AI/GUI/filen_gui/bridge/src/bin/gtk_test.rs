use gtk::prelude::*;

fn main() {
    gtk::init().unwrap();
    let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
    
    let targets = vec![
        gtk::TargetEntry::new("x-special/gnome-copied-files", gtk::TargetFlags::empty(), 0),
        gtk::TargetEntry::new("text/uri-list", gtk::TargetFlags::empty(), 1),
    ];

    let payload = "copy\nfile:///tmp/test.txt".to_string();
    let paths = vec!["/tmp/test.txt".to_string()];

    clipboard.set_with_data(&targets, move |_cb, sel_data, info| {
        if info == 0 {
            let data = payload.clone().into_bytes();
            sel_data.set(&gdk::Atom::intern("x-special/gnome-copied-files"), 8, &data);
        } else if info == 1 {
            let mut uri_list = String::new();
            for path in &paths {
                uri_list.push_str(&format!("file://{}\r\n", path));
            }
            let data = uri_list.into_bytes();
            sel_data.set(&gdk::Atom::intern("text/uri-list"), 8, &data);
        }
    });
}
