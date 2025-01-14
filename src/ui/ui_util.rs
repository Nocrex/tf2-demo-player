use adw::prelude::*;
use relm4::prelude::*;

pub async fn delete_dialog(root: &adw::Window) -> bool {
    let ad = adw::AlertDialog::builder()
        .default_response("cancel")
        .close_response("cancel")
        .body("Deleting selected demos!")
        .heading("Are you sure?")
        .build();

    ad.add_responses(&[("cancel", "Cancel"), ("delete", "Delete")]);
    ad.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

    match ad.choose_future(root).await.as_str() {
        "delete" => true,
        _ => false,
    }
}

pub async fn notice_dialog(root: &adw::Window, title: &str, message: &str) {
    let ad = adw::AlertDialog::builder()
        .default_response("ok")
        .close_response("ok")
        .body(message)
        .heading(title)
        .build();
    ad.add_response("ok", "OK");
    ad.choose_future(root).await;
}

pub async fn entry_dialog(
    root: &adw::Window,
    title: &str,
    message: &str,
    default_text: &str,
) -> Option<String> {
    let entry = gtk::Entry::new();
    entry.set_text(default_text);
    let ad = adw::AlertDialog::builder()
        .default_response("ok")
        .close_response("ok")
        .extra_child(&entry)
        .body(message)
        .heading(title)
        .build();
    ad.add_response("ok", "OK");
    ad.add_response("cancel", "Cancel");
    entry.grab_focus();
    match ad.choose_future(root).await.as_str() {
        "ok" => Some(entry.text().as_str().to_owned()),
        _ => None,
    }
}
