use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use adw::prelude::*;
use relm4::actions::RelmAction;
use relm4::actions::RelmActionGroup;
use relm4::prelude::*;

use crate::demo_manager::Event;
use crate::ui::about_window::AboutMsg;
use crate::ui::settings_window::*;
use crate::ui::util;
use crate::{
    demo_manager::{Demo, DemoManager},
    rcon_manager::RconManager,
    settings::Settings,
};
use demo_list::*;
use info_pane::InfoPaneMsg;

use super::about_window::AboutModel;
use info_pane::InfoPaneModel;
use info_pane::InfoPaneOut;

mod controls;
mod demo_infobox;
mod demo_list;
mod demo_object;
mod event_dialog;
mod event_list;
mod event_object;
mod info_pane;

#[derive(Debug)]
pub enum RconAction {
    Play(String),
    GotoTick(u32),
    GotoEvent(Event),
    Stop,
}

#[derive(Debug)]
pub enum DemoPlayerMsg {
    OpenSettings,
    SettingsClosed(Settings),
    ShowAbout,

    DeleteSelected,
    DeleteUnfinished,
    DeleteUnmarked,
    CleanReplays,

    OpenFolder(Option<std::path::PathBuf>, bool),
    SelectFolder,
    ReloadFolder,
    ShowSidebar,
    FavoriteFolder,

    DemosChanged(bool),

    Rcon(RconAction),
    PlayDemoDblclck(String),
    DemoSelected(Option<String>, bool),
    DemoSave(Demo),
    DemoUpdate(Demo),
}

relm4::new_action_group!(AppMenu, "app-menu");
relm4::new_stateless_action!(DeleteUnfinishedAction, AppMenu, "clean-unfinished");
relm4::new_stateless_action!(DeleteUnmarkedAction, AppMenu, "clean-unmarked");
relm4::new_stateless_action!(CleanReplaysAction, AppMenu, "clean-replays");

#[derive(Debug)]
pub enum DemoPlayerCmd {
    Progress(usize, usize),
    Done(std::path::PathBuf, bool),
}

pub struct DemoPlayerModel {
    demo_manager: Arc<Mutex<DemoManager>>,
    rcon_manager: RconManager,
    settings: Rc<RefCell<Settings>>,

    selected_demo: Option<Demo>,
    loading: Option<(usize, usize)>,

    preferences_wnd: Option<Controller<PreferencesModel>>,
    about_wnd: Controller<AboutModel>,

    demo_list: Controller<DemoListModel>,
    demo_details: Controller<InfoPaneModel>,
}

#[relm4::component(async pub)]
impl AsyncComponent for DemoPlayerModel {
    type Input = DemoPlayerMsg;
    type Output = ();
    type Init = ();
    type CommandOutput = DemoPlayerCmd;

    view! {
        #[name="main_window"]
        adw::Window {
            set_title: Some("Demo Player"),
            set_size_request: (1000, 850),
            set_icon_name: Some("tf2demoplayer"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar{
                    set_decoration_layout: Some(":minimize,maximise,close"),
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle{
                        set_title: "Demo Player",
                        #[watch]
                        set_subtitle: model.settings.borrow().demo_folder_path.as_ref().map_or("(unset)", |p|p.to_str().unwrap()),
                    },

                    pack_start = &gtk::Button{
                        set_icon_name: "open-menu-symbolic",
                        connect_clicked => DemoPlayerMsg::ShowSidebar,
                    },

                    pack_end = &adw::SplitButton{
                        #[watch]
                        set_sensitive: model.loading.is_none(),
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("Delete selected demo(s)"),
                        connect_clicked => DemoPlayerMsg::DeleteSelected,
                        set_menu_model: Some(&delete_menu),
                    },

                    pack_end = &gtk::Button{
                        #[watch]
                        set_sensitive: model.loading.is_none(),
                        set_icon_name: "view-refresh-symbolic",
                        set_tooltip_text: Some("Reload demo folder"),
                        connect_clicked => DemoPlayerMsg::ReloadFolder,
                    }
                },
                #[wrap(Some)]
                set_content: sidebar = &adw::OverlaySplitView{
                    set_collapsed: true,

                    #[wrap(Some)]
                    set_sidebar = &gtk::Box{
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 5,
                        gtk::Box{
                            set_halign: gtk::Align::Start,
                            gtk::Button{
                                #[watch]
                                set_sensitive: model.loading.is_none(),
                                add_css_class: "flat",
                                add_css_class: "circular",
                                set_icon_name: "folder-symbolic",
                                connect_clicked => DemoPlayerMsg::SelectFolder,
                                set_tooltip_text: Some("Open folder"),
                            },
                            gtk::Button{
                                #[watch]
                                set_icon_name: if model.settings.borrow().favorited() {relm4_icons::icon_names::STAR_LARGE} else {relm4_icons::icon_names::STAR_OUTLINE_ROUNDED},
                                add_css_class: "flat",
                                add_css_class: "circular",
                                connect_clicked => DemoPlayerMsg::FavoriteFolder,
                                set_tooltip_text: Some("Favorite current folder"),
                            },
                        },
                        gtk::ScrolledWindow{
                            #[watch]
                            set_sensitive: model.loading.is_none(),
                            set_vexpand: true,
                            #[watch]
                            set_child: Some(&{
                                let b = gtk::Box::new(gtk::Orientation::Vertical, 5);

                                for path in &model.settings.borrow().favorited_folders {
                                    let bu = gtk::Button::new();
                                    bu.set_label(&path.display().to_string());
                                    bu.child().unwrap().set_halign(gtk::Align::Start);
                                    bu.child().and_downcast_ref::<gtk::Label>().unwrap().set_wrap(true);
                                    bu.child().and_downcast_ref::<gtk::Label>().unwrap().set_wrap_mode(gtk::pango::WrapMode::WordChar);
                                    bu.child().unwrap().inline_css("font-weight: normal");
                                    let path = path.clone();
                                    let sender = sender.clone();
                                    bu.connect_clicked(move |_|{
                                        sender.input(DemoPlayerMsg::OpenFolder(Some(path.clone()), true));
                                    });
                                    bu.add_css_class("flat");
                                    b.append(&bu);
                                }

                                b
                            }),
                        },
                        gtk::Separator{
                            set_orientation: gtk::Orientation::Horizontal,
                            set_margin_top: 5,
                            set_margin_bottom: 5,
                        },
                        gtk::Box {
                            set_valign: gtk::Align::End,
                            set_halign: gtk::Align::Start,
                            gtk::Button{
                                set_icon_name: relm4_icons::icon_names::SETTINGS,
                                add_css_class: "flat",
                                add_css_class: "circular",
                                connect_clicked => DemoPlayerMsg::OpenSettings,
                                set_tooltip_text: Some("Settings"),
                            },
                            gtk::Button{
                                set_icon_name: relm4_icons::icon_names::INFO_OUTLINE,
                                add_css_class: "flat",
                                add_css_class: "circular",
                                connect_clicked => DemoPlayerMsg::ShowAbout,
                                set_tooltip_text: Some("About"),
                            },
                        }
                    },

                    #[wrap(Some)]
                    set_content: pane = &gtk::Paned{
                        set_orientation: gtk::Orientation::Vertical,
                        set_position: 400,
                        set_shrink_end_child: false,
                        set_shrink_start_child: false,

                        #[wrap(Some)]
                        set_start_child = &gtk::Overlay{
                            #[wrap(Some)]
                            set_child = model.demo_list.widget(),
                            add_overlay = &gtk::Box{
                                set_hexpand: true,
                                set_vexpand: true,
                                add_css_class: "view",
                                #[watch]
                                set_visible: model.loading.is_some(),
                                gtk::Box{
                                    set_halign: gtk::Align::Center,
                                    set_valign: gtk::Align::Center,
                                    set_hexpand: true,
                                    set_vexpand: true,
                                    set_orientation: gtk::Orientation::Vertical,
                                    gtk::Spinner{
                                       set_spinning: true,
                                    },
                                    gtk::Label{
                                        set_label: "Loading demos",
                                    },
                                    gtk::Label{
                                        #[watch]
                                        set_label: &format!("{}/{}", model.loading.map_or(0, |l|l.0), model.loading.map_or(0, |l|l.1))
                                    }
                                }
                            }
                        },

                        #[wrap(Some)]
                        set_end_child = model.demo_details.widget(),
                    }
                },
            }
        }
    }

    menu! {
        delete_menu: {
            "Delete 0s demos" => DeleteUnfinishedAction,
            "Delete demos without bookmarks" => DeleteUnmarkedAction,
            "Clean replays" => CleanReplaysAction,
        }
    }

    async fn init(
        _: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let settings = Rc::new(RefCell::new(Settings::load()));

        let demo_list = DemoListModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                DemoListOut::SelectionChanged(demo) => DemoPlayerMsg::DemoSelected(demo, false),
                DemoListOut::DemoActivated(name) => DemoPlayerMsg::PlayDemoDblclck(name),
            });

        let demo_details = InfoPaneModel::builder()
            .launch((root.clone(), settings.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                InfoPaneOut::Rcon(act) => DemoPlayerMsg::Rcon(act),
                InfoPaneOut::Save(demo) => DemoPlayerMsg::DemoSave(demo),
                InfoPaneOut::Update(demo) => DemoPlayerMsg::DemoUpdate(demo),
            });

        let about_wnd = AboutModel::builder().launch(root.clone()).detach();

        let model = Self {
            demo_manager: Arc::new(Mutex::new(DemoManager::new())),
            rcon_manager: RconManager::new(settings.clone().borrow().rcon_pw.to_owned()),
            settings: settings,
            preferences_wnd: None,
            about_wnd,
            demo_list,
            demo_details,
            selected_demo: None,
            loading: None,
        };

        let widgets = view_output!();

        #[cfg(debug_assertions)]
        widgets.main_window.add_css_class("devel");

        {
            let mut group = RelmActionGroup::<AppMenu>::new();

            let delete_unfinished_sender = sender.clone();
            let delete_unfinished_action: RelmAction<DeleteUnfinishedAction> =
                RelmAction::new_stateless(move |_| {
                    delete_unfinished_sender.input(DemoPlayerMsg::DeleteUnfinished);
                });
            group.add_action(delete_unfinished_action);

            let delete_unmarked_sender = sender.clone();
            let delete_unmarked_action: RelmAction<DeleteUnmarkedAction> =
                RelmAction::new_stateless(move |_| {
                    delete_unmarked_sender.input(DemoPlayerMsg::DeleteUnmarked);
                });
            group.add_action(delete_unmarked_action);

            let clean_replays_sender = sender.clone();
            let clean_replays_action: RelmAction<CleanReplaysAction> =
                RelmAction::new_stateless(move |_| {
                    clean_replays_sender.input(DemoPlayerMsg::CleanReplays);
                });
            group.add_action(clean_replays_action);

            let actions = group.into_action_group();
            widgets
                .main_window
                .insert_action_group("app-menu", Some(&actions));
        }

        if let Ok(Some(ver)) = crate::util::check_new_version()
            .await
            .inspect_err(|e| log::warn!("Failed to fetch newest version: {e:?}"))
        {
            util::notice_dialog(
                &root,
                &format!("New version available ({} -> {ver})", env!("CARGO_PKG_VERSION")),
                &format!("Visit the <a href=\"http://github.com/Nocrex/tf2-demo-player/releases/latest\">releases section</a> to download it"),
            );
        }

        sender.input(DemoPlayerMsg::OpenFolder(
            model.settings.borrow().demo_folder_path.clone(),
            true,
        ));

        AsyncComponentParts { model, widgets }
    }

    async fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            DemoPlayerMsg::DeleteUnfinished => {
                self.demo_manager.lock().unwrap().delete_empty_demos().await;
                sender.input(DemoPlayerMsg::DemosChanged(false));
            }
            DemoPlayerMsg::DeleteUnmarked => {
                self.demo_manager
                    .lock()
                    .unwrap()
                    .delete_unmarked_demos()
                    .await;
                sender.input(DemoPlayerMsg::DemosChanged(false));
            }
            DemoPlayerMsg::CleanReplays => 'replay_clean: {
                if self.settings.borrow().tf_folder_path.is_none() {
                    util::notice_dialog(
                        root,
                        "TF2 folder path not set up",
                        "Please check your TF2 folder setting",
                    );
                    break 'replay_clean;
                }
                let obsoletes = crate::util::find_obsolete_replays(
                    self.settings.borrow().replays_folder().unwrap(),
                )
                .await;
                if let Err(e) = obsoletes {
                    util::notice_dialog(root, "Error while loading replays", &e.to_string());
                } else if let Ok(obsolete_dmx_files) = obsoletes {
                    if obsolete_dmx_files.is_empty() {
                        util::notice_dialog(root, "No replays to clean", "");
                    } else {
                        if util::delete_dialog(root, obsolete_dmx_files.len()).await {
                            let res = async_std::task::spawn_blocking(|| {
                                trash::delete_all(obsolete_dmx_files)
                            })
                            .await;
                            if let Err(e) = res {
                                util::notice_dialog(root, "Error cleaning demos", &e.to_string());
                            }
                        };
                    }
                }
            }
            DemoPlayerMsg::OpenSettings => {
                self.preferences_wnd = Some(
                    PreferencesModel::builder()
                        .launch((self.settings.borrow().clone(), root.clone()))
                        .forward(sender.input_sender(), |po| match po {
                            PreferencesOut::Save(s) => DemoPlayerMsg::SettingsClosed(s),
                        }),
                );
                self.preferences_wnd
                    .as_ref()
                    .unwrap()
                    .emit(PreferencesMsg::Show);
            }
            DemoPlayerMsg::SettingsClosed(settings) => {
                self.settings.replace(settings);
                self.rcon_manager = RconManager::new(self.settings.borrow().rcon_pw.clone());
                self.preferences_wnd.take();
            }
            DemoPlayerMsg::ShowSidebar => {
                widgets
                    .sidebar
                    .set_show_sidebar(!widgets.sidebar.shows_sidebar());
            }
            DemoPlayerMsg::SelectFolder => {
                let dia = gtk::FileDialog::builder().build();
                let res = dia.select_folder_future(Some(root)).await;
                if let Ok(file) = res {
                    let path = file.path().unwrap();
                    sender.input(DemoPlayerMsg::OpenFolder(Some(path), true));
                }
            }
            DemoPlayerMsg::OpenFolder(path, scroll_up) => match path {
                None => self.demo_manager.lock().unwrap().clear(),
                Some(path) => {
                    let dm = self.demo_manager.clone();
                    self.loading = Some((0, 0));
                    sender.spawn_command(move |s| {
                        if path.exists() {
                            dm.lock().unwrap().load_demos(&path, |current, total| {
                                if current % 10 == 0 {
                                    s.emit(DemoPlayerCmd::Progress(current, total))
                                }
                            });
                        } else {
                            dm.lock().unwrap().clear();
                        }
                        s.emit(DemoPlayerCmd::Done(path, scroll_up))
                    });
                }
            },
            DemoPlayerMsg::ReloadFolder => {
                sender.input(DemoPlayerMsg::OpenFolder(
                    self.settings.borrow().demo_folder_path.clone(),
                    false,
                ));
            }
            DemoPlayerMsg::DemoSelected(opt_name, reselected) => {
                let mut demo = None::<Demo>;
                if let Some(name) = opt_name {
                    demo = self.demo_manager.lock().unwrap().get_demo(&name).cloned();
                }
                self.demo_details
                    .emit(InfoPaneMsg::Display(demo.clone(), reselected));
                self.selected_demo = demo;
            }
            DemoPlayerMsg::Rcon(act) => {
                // TODO: show status in UI
                match act {
                    RconAction::Play(name) => {
                        let dm = self.demo_manager.lock().unwrap();
                        let demo = dm.get_demo(&name).unwrap();
                        let _ = self.rcon_manager.play_demo(demo).await;
                    }
                    RconAction::GotoTick(tick) => {
                        let _ = self
                            .rcon_manager
                            .skip_to_tick(tick, self.settings.borrow().pause_after_seek)
                            .await;
                    }
                    RconAction::GotoEvent(ev) => {
                        let _ = self
                            .rcon_manager
                            .skip_to_tick(
                                (ev.tick
                                    - (self.settings.borrow().event_skip_predelay
                                        * self.selected_demo.as_ref().unwrap().tps())
                                    .round() as u32)
                                    .clamp(
                                        0,
                                        self.selected_demo
                                            .as_ref()
                                            .unwrap()
                                            .header
                                            .as_ref()
                                            .map_or(0, |h| h.ticks),
                                    ),
                                true,
                            )
                            .await;
                    }
                    RconAction::Stop => {
                        let _ = self.rcon_manager.stop_playback().await;
                    }
                }
            }
            DemoPlayerMsg::PlayDemoDblclck(name) => {
                if self.settings.borrow().doubleclick_play {
                    sender.input(DemoPlayerMsg::Rcon(RconAction::Play(name)));
                }
            }
            DemoPlayerMsg::DeleteSelected => {
                let count = self.demo_list.model().get_selected_demos().len();
                if util::delete_dialog(root, count).await {
                    for d in self.demo_list.model().get_selected_demos() {
                        self.demo_manager.lock().unwrap().delete_demo(&d).await;
                    }
                    sender.input(DemoPlayerMsg::DemosChanged(false));
                }
            }
            DemoPlayerMsg::DemosChanged(scroll) => {
                self.demo_list.emit(DemoListMsg::Update(
                    self.demo_manager.lock().unwrap().get_demos().clone(),
                    scroll,
                ));
            }
            DemoPlayerMsg::DemoSave(demo) => {
                let name = demo.filename.clone();
                demo.save_json().await;
                self.demo_manager
                    .lock()
                    .unwrap()
                    .get_demos_mut()
                    .insert(name.clone(), demo);
                sender.input(DemoPlayerMsg::DemoSelected(Some(name), true));
                sender.input(DemoPlayerMsg::DemosChanged(false));
            }
            DemoPlayerMsg::DemoUpdate(demo) => {
                self.demo_manager
                    .lock()
                    .unwrap()
                    .get_demos_mut()
                    .insert(demo.filename.clone(), demo);
            }
            DemoPlayerMsg::FavoriteFolder => {
                self.settings.borrow_mut().toggle_favorite();
            }
            DemoPlayerMsg::ShowAbout => {
                self.about_wnd.emit(AboutMsg::Open);
            }
        }
        self.update_view(widgets, sender);
    }

    async fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            DemoPlayerCmd::Progress(current, total) => {
                self.loading = Some((current, total));
            }
            DemoPlayerCmd::Done(path, scroll_up) => {
                self.settings.borrow_mut().folder_opened(&path);
                self.settings.borrow().save();
                self.loading = None;
                self.demo_details.emit(InfoPaneMsg::Display(None, false));
                sender.input(DemoPlayerMsg::DemosChanged(scroll_up));
            }
        }
    }
}
