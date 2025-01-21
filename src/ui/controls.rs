use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use relm4::prelude::*;
use relm4_icons::icon_names;

use crate::demo_manager::Demo;
use crate::settings::Settings;
use crate::util::sec_to_timestamp;
use crate::util::ticks_to_sec;

use super::inspection_window::InspectionModel;
use super::inspection_window::InspectionOut;
use super::ui_util;
use super::window::RconAction;

#[derive(Debug)]
pub enum ControlsOut {
    Rcon(RconAction),
    Inspect(String),

    SaveChanges,
    DiscardChanges,
    PlayheadMoved(u32),
}

#[derive(Debug)]
pub enum ControlsMsg {
    SetDemo(Option<Demo>, bool),
    SetDirty(bool),

    PlayheadMoved(f64),
    Play,
    Stop,
    GotoPlayhead,
    SeekForward,
    SeekBackward,
    ConvertReplay,
    InspectDemo,

    SaveChanges,
    DiscardChanges,
}

pub struct ControlsModel {
    dirty: bool,
    demo: Option<Demo>,
    playhead_time: f64,

    window: adw::Window,
    settings: Rc<RefCell<Settings>>,

    inspection_wnd: AsyncController<InspectionModel>,
}

#[relm4::component(async pub)]
impl AsyncComponent for ControlsModel {
    type Init = (adw::Window, Rc<RefCell<Settings>>);
    type Input = ControlsMsg;
    type Output = ControlsOut;
    type CommandOutput = std::sync::Arc<tf_demo_parser::MatchState>;

    view! {
        gtk::Grid {
            set_column_homogeneous: false,
            set_margin_end: 5,
            set_margin_start: 5,
            set_margin_bottom: 5,

            attach[1,0,1,1]: playhead = &gtk::Scale {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                #[watch]
                #[block_signal(ph_handler)]
                set_value: model.playhead_time,
                connect_value_changed[sender] => move |ph| {
                    sender.input(ControlsMsg::PlayheadMoved(ph.value()));
                } @ph_handler,
                set_adjustment = &gtk::Adjustment {
                    set_step_increment: 1.0,
                    set_lower: 0.0,
                    #[watch]
                    set_upper?: model.demo.as_ref().and_then(|d|d.header.as_ref()).map(|h|h.ticks.into()),
                }
            },

            attach[0,0,1,1] = &gtk::Label {
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Start,
                set_justify: gtk::Justification::Center,
                set_width_request: 60,
                set_selectable: true,
                set_margin_top: 10,
                set_margin_bottom: 10,
                #[watch]
                set_label: &format!("{}\n{}",
                    sec_to_timestamp(
                        ticks_to_sec(
                            model.playhead_time as u32,
                            model.demo.as_ref().map(|d|d.tps()).unwrap_or(Demo::TICKRATE)
                        )),
                    model.playhead_time as u64),
            },

            attach[2,0,1,1] = &gtk::Label {
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Start,
                set_justify: gtk::Justification::Center,
                set_width_request: 60,
                set_selectable: true,
                set_margin_top: 10,
                set_margin_bottom: 10,
                #[watch]
                set_label: &format!("{}\n{}",
                    sec_to_timestamp(
                        ticks_to_sec(
                            model.demo.as_ref().and_then(|d|d.header.as_ref()).map_or(0,|h|h.ticks),
                            model.demo.as_ref().map(|d|d.tps()).unwrap_or(Demo::TICKRATE)
                        )
                    ),
                    model.demo.as_ref().and_then(|d|d.header.as_ref()).map_or(0,|h|h.ticks),
                ),
            },

            attach[0,1,3,1] = &gtk::CenterBox {
                #[wrap(Some)]
                set_start_widget = &gtk::Box{
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Button{
                        set_icon_name: "media-playback-start-symbolic",
                        set_tooltip_text: Some("Play demo"),
                        connect_clicked => ControlsMsg::Play,
                    },

                    gtk::Button{
                        set_icon_name: "find-location-symbolic",
                        set_tooltip_text: Some("Skip to tick"),
                        connect_clicked => ControlsMsg::GotoPlayhead,
                    },

                    gtk::Button{
                        set_icon_name: "media-playback-stop-symbolic",
                        set_tooltip_text: Some("Stop Playback"),
                        connect_clicked => ControlsMsg::Stop,
                    },

                    gtk::Separator{
                        set_orientation: gtk::Orientation::Vertical,
                    },

                    gtk::Button{
                        set_icon_name: icon_names::SKIP_BACKWARDS_30,
                        set_tooltip_text: Some("-30s"),
                        connect_clicked => ControlsMsg::SeekBackward,
                    },

                    gtk::Button{
                        set_icon_name: icon_names::SKIP_FORWARD_30,
                        set_tooltip_text: Some("+30s"),
                        connect_clicked => ControlsMsg::SeekForward,
                    },

                    gtk::Separator{
                        set_orientation: gtk::Orientation::Vertical,
                    },

                    gtk::Button{
                        set_icon_name: icon_names::VIDEO_CLIP,
                        set_tooltip_text: Some("Convert to replay"),
                        connect_clicked => ControlsMsg::ConvertReplay,
                    },

                    gtk::Button{
                        set_icon_name: icon_names::LIST_COMPACT,
                        set_tooltip_text: Some("Inspect demo"),
                        connect_clicked => ControlsMsg::InspectDemo,
                    }
                },

                #[wrap(Some)]
                set_end_widget = &gtk::Box{
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    #[watch]
                    set_sensitive: model.dirty,

                    gtk::Button{
                        set_icon_name: icon_names::CROSS_SMALL_CIRCLE_OUTLINE,
                        set_tooltip_text: Some("Discard changes"),
                        connect_clicked => ControlsMsg::DiscardChanges,
                    },

                    gtk::Button{
                        set_icon_name: "document-save-symbolic",
                        set_tooltip_text: Some("Save changes"),
                        connect_clicked => ControlsMsg::SaveChanges,
                    },

                }
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = ControlsModel {
            demo: None,
            dirty: false,
            playhead_time: 0.0,
            window: init.0,
            settings: init.1,
            inspection_wnd: InspectionModel::builder().launch(()).forward(
                sender.input_sender(),
                |msg| match msg {
                    InspectionOut::GotoTick(tick) => ControlsMsg::PlayheadMoved(tick.into()),
                },
            ),
        };

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _: &Self::Root,
    ) {
        //log::debug!("{:?}", message);
        match message {
            ControlsMsg::PlayheadMoved(val) => {
                self.playhead_time = val;
                let _ = sender.output(ControlsOut::PlayheadMoved(val as u32));
            }
            ControlsMsg::SetDemo(dem, keep_playhead) => {
                if !keep_playhead {
                    self.playhead_time = 0.0;
                }
                widgets.playhead.clear_marks();
                for event in dem.as_ref().map_or(&vec![], |d| &d.events) {
                    widgets
                        .playhead
                        .add_mark(event.tick as f64, gtk::PositionType::Bottom, None);
                }
                self.demo = dem;
                self.dirty = false;
            }
            ControlsMsg::Play => {
                let _ = sender.output(ControlsOut::Rcon(RconAction::Play(
                    self.demo.as_ref().unwrap().filename.clone(),
                )));
            }
            ControlsMsg::GotoPlayhead => {
                let _ = sender.output(ControlsOut::Rcon(RconAction::GotoTick(
                    self.playhead_time as u32,
                )));
            }
            ControlsMsg::Stop => {
                let _ = sender.output(ControlsOut::Rcon(RconAction::Stop));
            }
            ControlsMsg::SeekBackward => {
                self.playhead_time -= 30.0
                    * self
                        .demo
                        .as_ref()
                        .map(|d| d.tps())
                        .unwrap_or(Demo::TICKRATE) as f64;
                self.playhead_time = self
                    .playhead_time
                    .clamp(0.0, widgets.playhead.adjustment().upper());
            }
            ControlsMsg::SeekForward => {
                self.playhead_time += 30.0
                    * self
                        .demo
                        .as_ref()
                        .map(|d| d.tps())
                        .unwrap_or(Demo::TICKRATE) as f64;
                self.playhead_time = self
                    .playhead_time
                    .clamp(0.0, widgets.playhead.adjustment().upper());
            }
            ControlsMsg::ConvertReplay => 'replay: {
                if let Some(demo) = &mut self.demo {
                    let tf_folder_path =
                        async_std::path::PathBuf::from(&self.settings.borrow().tf_folder_path);
                    if !tf_folder_path.is_dir().await {
                        ui_util::notice_dialog(
                            &self.window,
                            "TF2 folder does not exist or cannot be accessed",
                            "Please check your TF2 folder setting",
                        )
                        .await;
                        break 'replay;
                    }
                    let replay_folder: async_std::path::PathBuf =
                        self.settings.borrow().replays_folder().into();
                    if !replay_folder.is_dir().await {
                        ui_util::notice_dialog(
                            &self.window,
                            "Replay folder does not exist or cannot be accessed",
                            &format!(
                                "Please check your TF2 folder setting\n({})",
                                replay_folder.to_str().unwrap()
                            ),
                        )
                        .await;
                        break 'replay;
                    }
                    if demo.has_replay(&replay_folder).await {
                        ui_util::notice_dialog(&self.window, "Demo already converted", "").await;
                        break 'replay;
                    }
                    if let Some(title) = ui_util::entry_dialog(
                        &self.window,
                        "Replay title",
                        "Title to save the replay under",
                        &demo.filename,
                    )
                    .await
                    {
                        match demo.convert_to_replay(&replay_folder, &title).await {
                            Ok(_) => {
                                ui_util::notice_dialog(
                                    &self.window,
                                    "Replay created successfully",
                                    "",
                                )
                                .await
                            }
                            Err(e) => {
                                ui_util::notice_dialog(
                                    &self.window,
                                    "Failed to create replay",
                                    &e.to_string(),
                                )
                                .await
                            }
                        };
                    }
                }
            }
            ControlsMsg::InspectDemo => {
                /*let _ = sender.output(ControlsOut::Inspect(
                    self.demo.as_ref().unwrap().filename.clone(),
                ));*/
                let demo_clone = self.demo.clone().unwrap();
                self.inspection_wnd.emit(demo_clone);
            }
            ControlsMsg::DiscardChanges => {
                let _ = sender.output(ControlsOut::DiscardChanges);
                self.dirty = false;
            }
            ControlsMsg::SaveChanges => {
                let _ = sender.output(ControlsOut::SaveChanges);
                self.dirty = false;
            }
            ControlsMsg::SetDirty(state) => self.dirty = state,
        }
        self.update_view(widgets, sender);
    }
}
