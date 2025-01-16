use adw::prelude::*;
use relm4::actions::RelmAction;
use relm4::actions::RelmActionGroup;
use relm4::prelude::*;

use crate::ui::info_pane::InfoPaneMsg;
use crate::ui::settings_window::*;
use crate::ui::demo_list::*;
use crate::{demo_manager::{DemoManager, Demo}, rcon_manager::RconManager, settings::Settings};

use super::info_pane::InfoPaneModel;

#[derive(Debug)]
pub enum RconAction {
    Play(String),
    Goto(u32),
    Stop,
}

#[derive(Debug)]
pub enum DemoPlayerMsg{
    OpenSettings,
    SettingsClosed(Settings),

    DeleteSelected,
    DeleteUnfinished,
    DeleteUnmarked,

    OpenFolder(String, bool),
    SelectFolder,
    ReloadFolder,
    
    DemosChanged(bool),
    
    Rcon(RconAction),
    PlayDemoDblclck(String),
    DemoSelected(Option<String>),
}

relm4::new_action_group!(AppMenu, "app-menu");
relm4::new_stateless_action!(OpenSettingsAction, AppMenu, "open-settings");
relm4::new_stateless_action!(DeleteUnfinishedAction, AppMenu, "clean-unfinished");
relm4::new_stateless_action!(DeleteUnmarkedAction, AppMenu, "clean-unmarked");

pub struct DemoPlayerModel{
    demo_manager: DemoManager,
    rcon_manager: RconManager,
    settings: Settings,

    //preferences_wnd: Controller<PreferencesModel>,
    demo_list: Controller<DemoListModel>,
    demo_details: Controller<InfoPaneModel>,
}

#[relm4::component(async pub)]
impl AsyncComponent for DemoPlayerModel {
    type Input = DemoPlayerMsg;
    type Output = ();
    type Init = ();
    type CommandOutput = ();

    view! {
        #[name="main_window"]
        adw::Window {
            set_title: Some("Demo Player"),
            set_size_request: (1000, 850),
            
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar{
                    set_title_widget: Some(&gtk::Label::new(
                        Some("Demo Player")
                    )),

                    pack_start = &adw::SplitButton{
                        set_icon_name: "folder-symbolic",
                        set_tooltip_text: Some("Select demo folder"),
                        connect_clicked => DemoPlayerMsg::SelectFolder,
                    },

                    pack_end: app_menu_button = &gtk::MenuButton{
                        set_icon_name: "open-menu-symbolic",
                        set_menu_model: Some(&app_menu)
                    },

                    pack_end = &gtk::Button{
                        set_icon_name: "edit-delete-symbolic",
                        set_tooltip_text: Some("Delete selected demo(s)"),
                        connect_clicked => DemoPlayerMsg::DeleteSelected,
                    },

                    pack_end = &gtk::Button{
                        set_icon_name: "view-refresh-symbolic",
                        set_tooltip_text: Some("Reload demo folder"),
                        connect_clicked => DemoPlayerMsg::ReloadFolder,
                    }
                },
                #[wrap(Some)]
                set_content: pane = &gtk::Paned{
                    set_orientation: gtk::Orientation::Vertical,
                    set_position: 400,
                    set_shrink_end_child: false,
                    set_shrink_start_child: false,

                    #[wrap(Some)]
                    set_start_child = model.demo_list.widget(),
                    
                    #[wrap(Some)]
                    set_end_child = model.demo_details.widget(),
                }
            }
        }
    }
    
    menu!{
        app_menu: {
            "Settings" => OpenSettingsAction,
            "Delete 0s demos" => DeleteUnfinishedAction,
            "Delete demos without bookmarks" => DeleteUnmarkedAction,
        }
    }
    
    async fn init(
            _: Self::Init,
            root: Self::Root,
            sender: AsyncComponentSender<Self>,
        ) -> AsyncComponentParts<Self> {
        
        let demo_list = DemoListModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg|{
                match msg {
                    DemoListOut::SelectionChanged(demo) => DemoPlayerMsg::DemoSelected(demo),
                    DemoListOut::DemoActivated(name) => DemoPlayerMsg::PlayDemoDblclck(name),
                }
            });
        
        let demo_details = InfoPaneModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg|{
               todo!() 
            });

        let settings = Settings::load();

        let model = Self { 
            demo_manager: DemoManager::new(), 
            rcon_manager: RconManager::new(settings.rcon_pw.to_owned()),
            settings: settings,
            /*preferences_wnd: PreferencesModel::builder()
            .transient_for(&root)
            .launch(settings)
            .forward(sender.input_sender(), |po|{
                match po {
                    PreferencesOut::Save(s) => DemoPlayerMsg::SettingsClosed(s)
                }
            }),*/
            demo_list,
            demo_details,
        };
        
        let widgets = view_output!();

        {
            let mut group = RelmActionGroup::<AppMenu>::new();
            
            let settings_sender = sender.clone();
            let settings_action: RelmAction<OpenSettingsAction> = RelmAction::new_stateless(move |_|{
                settings_sender.input(DemoPlayerMsg::OpenSettings);
            });
            group.add_action(settings_action);

            let delete_unfinished_sender = sender.clone();
            let delete_unfinished_action: RelmAction<DeleteUnfinishedAction> = RelmAction::new_stateless(move |_|{
                delete_unfinished_sender.input(DemoPlayerMsg::DeleteUnfinished);
            });
            group.add_action(delete_unfinished_action);
        
            let delete_unmarked_sender = sender.clone();
            let delete_unmarked_action: RelmAction<DeleteUnmarkedAction> = RelmAction::new_stateless(move |_|{
                delete_unmarked_sender.input(DemoPlayerMsg::DeleteUnmarked);
            });
            group.add_action(delete_unmarked_action);
        
            let actions = group.into_action_group();
            widgets.main_window.insert_action_group("app-menu", Some(&actions));
        }

        sender.input(DemoPlayerMsg::OpenFolder(model.settings.demo_folder_path.to_owned(), true));

        AsyncComponentParts{model, widgets}
    }
    
    async fn update(&mut self, message: Self::Input, sender: AsyncComponentSender<Self>, root: &Self::Root) {
        log::debug!("{:?}", message);
        match message {
            DemoPlayerMsg::DeleteUnfinished => {
                self.demo_manager.delete_empty_demos().await;
                sender.input(DemoPlayerMsg::DemosChanged(false));
            },
            DemoPlayerMsg::DeleteUnmarked => {
                self.demo_manager.delete_unmarked_demos().await;
                sender.input(DemoPlayerMsg::DemosChanged(false));
            },
            DemoPlayerMsg::OpenSettings =>{
                //self.preferences_wnd.emit(PreferencesMsg::Show);
            }
            DemoPlayerMsg::SettingsClosed(settings) => {
                self.settings = settings;
                self.rcon_manager = RconManager::new(self.settings.rcon_pw.clone());
            },
            DemoPlayerMsg::SelectFolder => {
                let dia = gtk::FileDialog::builder().build();
                let res = dia.select_folder_future(Some(root)).await;
                if let Ok(file) = res{
                    let path = file.path().unwrap().display().to_string();
                    sender.input(DemoPlayerMsg::OpenFolder(path, true));
                }
            },
            DemoPlayerMsg::OpenFolder(path, scroll_up) => {
                self.demo_manager.load_demos(&path).await;

                self.settings.demo_folder_path = path.clone();
                self.settings.recent_folders.retain(|p| *p != path);
                self.settings.recent_folders.insert(0, path);
                self.settings.recent_folders.truncate(5);
                self.demo_details.emit(InfoPaneMsg::Display(None));
                sender.input(DemoPlayerMsg::DemosChanged(scroll_up));
                // TODO: update recent folders menu
            },
            DemoPlayerMsg::ReloadFolder => {
                sender.input(DemoPlayerMsg::OpenFolder(self.settings.demo_folder_path.clone(), false));
            },
            DemoPlayerMsg::DemoSelected(opt_name) => {
                let mut demo = None::<Demo>;
                if let Some(name) = opt_name {
                    demo = self.demo_manager.get_demo(&name).cloned();
                }
                self.demo_details.emit(InfoPaneMsg::Display(demo));
            },
            DemoPlayerMsg::Rcon(act) => {
                match act {
                    RconAction::Play(name) => {
                        let demo = self.demo_manager.get_demo(&name).unwrap();
                        let _ = self.rcon_manager.play_demo(demo).await; // TODO: HANDLE AND OUTPUT
                    },
                    RconAction::Goto(tick) => {
                        let _ = self.rcon_manager.skip_to_tick(tick, true).await;
                    },
                    RconAction::Stop => {
                        let _ = self.rcon_manager.stop_playback().await;
                    }
                }
            },
            DemoPlayerMsg::PlayDemoDblclck(name) => {
                if self.settings.doubleclick_play {
                    sender.input(DemoPlayerMsg::Rcon(RconAction::Play(name)));
                }
            },
            DemoPlayerMsg::DeleteSelected => {
                let ad = adw::MessageDialog::builder()
                    .default_response("cancel")
                    .close_response("cancel")
                    .body("Deleting selected demos!")
                    .heading("Are you sure?")
                    .transient_for(root)
                    .build();

                ad.add_responses(&[("cancel", "Cancel"), ("delete", "Delete")]);
                ad.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                
                if ad.choose_future().await.as_str() == "delete"{
                    for d in self.demo_list.model().get_selected_demos() {
                        self.demo_manager.delete_demo(&d).await;
                    }
                    sender.input(DemoPlayerMsg::DemosChanged(false));
                }
            },
            DemoPlayerMsg::DemosChanged(scroll) => {
                self.demo_list.emit(DemoListMsg::Update(self.demo_manager.get_demos().clone(), scroll));
            }
        }
    }
}