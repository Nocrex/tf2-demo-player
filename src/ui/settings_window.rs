use adw::prelude::*;
use relm4::prelude::*;

use crate::{rcon_manager::RconManager, settings::Settings};

#[derive(Debug)]
pub enum PreferencesMsg {
    Show,
    ConnectionTest(String),
    Close,

    DoubleclickPlay(bool),
    EventSkipOffset(f64),
    TF2FolderPath,
    RConPassword(String),
}

#[derive(Debug)]
pub enum PreferencesOut {
    Save(Settings),
}

pub struct PreferencesModel {
    parent: adw::Window,

    settings: Settings,
    connection_test_msg: String,
    connection_test_active: bool,
}

#[derive(Debug)]
pub enum PreferencesCmd {
    ConnectionTestResult(String),
    FolderBrowseResult(std::path::PathBuf),
}

#[relm4::component(pub)]
impl Component for PreferencesModel {
    type Init = (Settings, adw::Window);
    type Input = PreferencesMsg;
    type Output = PreferencesOut;
    type CommandOutput = PreferencesCmd;

    view! {
        adw::PreferencesDialog{
            set_search_enabled: false,
            connect_closed[sender] => move |_| {
                sender.input(PreferencesMsg::Close);
            },

            add = &adw::PreferencesPage {
                set_icon_name: Some(&"preferences-system-symbolic"),
                set_title: "General",

                adw::PreferencesGroup {
                    set_title: "General",

                    adw::SwitchRow {
                        set_title: "Doubleclick to play demo",
                        set_active: model.settings.doubleclick_play,
                        connect_active_notify[sender] => move |sr| {
                            sender.input(PreferencesMsg::DoubleclickPlay(sr.is_active()));
                        }
                    },

                    adw::SpinRow {
                        set_title: "Event skip offset",
                        set_subtitle: "How many seconds before the even the playback should start",
                        set_digits: 1,
                        #[wrap(Some)]
                        set_adjustment = &gtk::Adjustment {
                            set_lower: -300.0,
                            set_upper: 300.0,
                            set_page_increment: 1.0,
                            set_step_increment: 0.1,
                            set_value: model.settings.event_skip_predelay.into(),
                            connect_value_changed[sender] => move |adj| {
                                sender.input(PreferencesMsg::EventSkipOffset(adj.value()));
                            },
                        }
                    },

                    adw::ActionRow {
                        set_title: "TF2 folder",
                        set_tooltip_text: Some("Folder that contains the \"tf\" folder, if set incorrectly replays will not show up in-game!"),
                        #[watch]
                        set_subtitle: model.settings.tf_folder_path.as_ref().map_or("(unset)", |p|p.to_str().unwrap()),
                        set_subtitle_selectable: true,
                        set_activatable_widget: Some(&tf_browse_button),

                        add_suffix: tf_browse_button = &gtk::Button {
                            set_valign: gtk::Align::Center,
                            set_label: "Browse",
                            connect_clicked => PreferencesMsg::TF2FolderPath,
                        }
                    },
                },
                adw::PreferencesGroup {
                    set_title: "RCon",

                    adw::PasswordEntryRow {
                        set_title: "RCon password",
                        set_text: &model.settings.rcon_pw,
                        connect_changed[sender] => move |per|{
                            sender.input(PreferencesMsg::RConPassword(per.text().as_str().to_owned()))
                        }
                    },

                    adw::ActionRow {
                        set_title: "Connection Test",
                        set_subtitle_selectable: true,
                        set_activatable_widget: Some(&connection_test_button),
                        #[watch]
                        set_subtitle: &model.connection_test_msg,

                        add_suffix: connection_test_button = &gtk::Button {
                            set_valign: gtk::Align::Center,
                            set_label: "Test",
                            #[watch]
                            set_sensitive: !model.connection_test_active,
                            connect_clicked[sender, pw = model.settings.rcon_pw.clone()] => move |_|{
                                sender.input(PreferencesMsg::ConnectionTest(pw.clone()))
                            }
                        }
                    }
                }
            },
        }
    }

    fn init(
        (settings, parent): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = PreferencesModel {
            settings,
            parent,
            connection_test_msg: "".to_owned(),
            connection_test_active: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            PreferencesMsg::ConnectionTest(pw) => sender.oneshot_command(async move {
                let mut manager = RconManager::new(pw);
                let res = manager.connect().await;
                PreferencesCmd::ConnectionTestResult(match res {
                    Ok(_) => "Connection Successful!".to_owned(),
                    Err(e) => match e.downcast().unwrap() {
                        rcon::Error::Auth => {
                            "Authorization failed, probably incorrect password".to_owned()
                        }
                        rcon::Error::CommandTooLong => "Command too long?".to_owned(),
                        rcon::Error::Io(e) => format!("Connection error: {:?}", e),
                    },
                })
            }),
            PreferencesMsg::Show => {
                self.connection_test_msg = "".to_owned();
                root.present(Some(&self.parent));
            }
            PreferencesMsg::Close => {
                self.settings.save();
                let _ = sender.output(PreferencesOut::Save(self.settings.clone()));
            }
            PreferencesMsg::DoubleclickPlay(p) => self.settings.doubleclick_play = p,
            PreferencesMsg::EventSkipOffset(off) => self.settings.event_skip_predelay = off as f32,
            PreferencesMsg::RConPassword(pass) => self.settings.rcon_pw = pass,
            PreferencesMsg::TF2FolderPath => {
                let dia = gtk::FileDialog::new();
                let sender = sender.clone();
                dia.select_folder(
                    Some(&self.parent),
                    None::<&gtk::gio::Cancellable>,
                    move |res| match res {
                        Ok(file) => sender
                            .command_sender()
                            .emit(PreferencesCmd::FolderBrowseResult(file.path().unwrap())),
                        Err(e) => log::warn!("Error while picking folder: {e}"),
                    },
                );
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            PreferencesCmd::ConnectionTestResult(msg) => {
                self.connection_test_msg = msg;
                self.connection_test_active = false;
            }
            PreferencesCmd::FolderBrowseResult(path) => {
                if !path.join("tf").is_dir() {
                    crate::ui::ui_util::notice_dialog(
                        &self.parent,
                        "Invalid folder selected",
                        "Please select the \"Team Fortress 2\" folder",
                    );
                } else {
                    self.settings.tf_folder_path = Some(path);
                }
            }
        }
    }
}
