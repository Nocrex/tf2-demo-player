use adw::prelude::*;
use relm4::prelude::*;

use crate::demo_manager::Demo;
use crate::util::sec_to_timestamp;
use crate::util::ticks_to_sec;

use super::window::RconAction;

#[derive(Debug)]
pub enum ControlsOut {
    Rcon(RconAction),
    ConvertReplay(String),
    Inspect(String),

    SaveChanges,
    DiscardChanges,
}

#[derive(Debug)]
pub enum ControlsMsg {
    SetDemo(Option<Demo>),
    SetDirty,

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

#[derive(Default)]
pub struct ControlsModel {
    dirty: bool,
    demo: Option<Demo>,
    playhead_time: f64,
    playhead: gtk::Scale,
}

#[relm4::component(pub)]
impl SimpleComponent for ControlsModel {
    type Init = ();
    type Input = ControlsMsg;
    type Output = ControlsOut;

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
                        set_icon_name: "media-seek-backward-symbolic",
                        set_tooltip_text: Some("-30s"),
                        connect_clicked => ControlsMsg::SeekBackward,
                    },

                    gtk::Button{
                        set_icon_name: "media-seek-forward-symbolic",
                        set_tooltip_text: Some("+30s"),
                        connect_clicked => ControlsMsg::SeekForward,
                    },

                    gtk::Separator{
                        set_orientation: gtk::Orientation::Vertical,
                    },

                    gtk::Button{
                        set_icon_name: "document-send-symbolic",
                        set_tooltip_text: Some("Convert to replay"),
                        connect_clicked => ControlsMsg::ConvertReplay,
                    },

                    gtk::Button{
                        set_icon_name: "view-list-symbolic",
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
                        set_icon_name: "edit-clear-all-symbolic",
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

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = ControlsModel::default();

        let widgets = view_output!();

        model.playhead = widgets.playhead.clone();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ControlsMsg::PlayheadMoved(val) => self.playhead_time = val,
            ControlsMsg::SetDemo(dem) => {
                self.playhead_time = 0.0;
                self.playhead.clear_marks();
                for event in dem.as_ref().map_or(&vec![], |d| &d.events) {
                    self.playhead
                        .add_mark(event.tick as f64, gtk::PositionType::Bottom, None);
                }
                self.demo = dem;
            }
            ControlsMsg::Play => {
                let _ = sender.output(ControlsOut::Rcon(RconAction::Play(
                    self.demo.as_ref().unwrap().filename.clone(),
                )));
            }
            ControlsMsg::GotoPlayhead => {
                let _ = sender.output(ControlsOut::Rcon(RconAction::Goto(
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
                    .clamp(0.0, self.playhead.adjustment().upper());
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
                    .clamp(0.0, self.playhead.adjustment().upper());
            }
            ControlsMsg::ConvertReplay => {
                let _ = sender.output(ControlsOut::ConvertReplay(
                    self.demo.as_ref().unwrap().filename.clone(),
                ));
            }
            ControlsMsg::InspectDemo => {
                let _ = sender.output(ControlsOut::Inspect(
                    self.demo.as_ref().unwrap().filename.clone(),
                ));
            }
            ControlsMsg::DiscardChanges => {
                let _ = sender.output(ControlsOut::DiscardChanges);
                self.dirty = false;
            }
            ControlsMsg::SaveChanges => {
                let _ = sender.output(ControlsOut::SaveChanges);
                self.dirty = false;
            }
            ControlsMsg::SetDirty => self.dirty = true,
        }
    }
}
