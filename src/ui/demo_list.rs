use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::RandomState;

use adw::prelude::*;
use chrono::TimeZone;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use relm4::factory::Position;
use relm4::prelude::*;

use crate::demo_manager::Demo;
use crate::ui::demo_object::DemoObject;

pub struct DemoListModel {
    list_model: gio::ListStore,
    list_selection: gtk::MultiSelection,
}

#[derive(Debug)]
pub enum DemoListMsg {
    Update(HashMap<String, Demo>, bool),
    SelectionChanged,
}

#[derive(Debug)]
pub enum DemoListOut {
    SelectionChanged(Option<String>),
    DemoActivated(String),
}

impl DemoListModel {
    pub fn get_selected_demos(&self) -> Vec<String> {
        let selected = self.list_selection.selection();
        if selected.is_empty() {
            return vec![];
        }

        let model = self.list_selection.model().unwrap();

        (0..selected.size() as u32)
            .map(|i| {
                model
                    .item(selected.nth(i))
                    .and_downcast_ref::<DemoObject>()
                    .unwrap()
                    .name()
            })
            .collect()
    }
}

#[relm4::component(pub)]
impl Component for DemoListModel {
    type Init = ();
    type Input = DemoListMsg;
    type Output = DemoListOut;
    type CommandOutput = ();

    view! {
        gtk::ScrolledWindow{
            set_has_frame: true,
            set_hscrollbar_policy: gtk::PolicyType::Automatic,

            #[name="demo_list"]
            gtk::ColumnView{
                set_model: Some(&model.list_selection),
                connect_activate[sender] => move |view,ind| {
                    let demo_name = view.model().unwrap().item(ind).and_downcast_ref::<DemoObject>().unwrap().name();
                    let _ = sender.output(DemoListOut::DemoActivated(demo_name));
                }
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let liststore = gio::ListStore::new::<DemoObject>();
        let sorted_model = gtk::SortListModel::builder().model(&liststore).build();

        let model = DemoListModel {
            list_model: liststore.clone(),
            list_selection: gtk::MultiSelection::new(None::<gio::ListStore>),
        };

        model.list_selection.set_model(Some(&sorted_model));

        {
            let sender = sender.clone();
            model
                .list_selection
                .connect_selection_changed(move |_, _, _| {
                    sender.input(DemoListMsg::SelectionChanged);
                });
        }

        let widgets = view_output!();

        sorted_model.set_sorter(widgets.demo_list.sorter().as_ref());

        let name_factory = gtk::SignalListItemFactory::new();
        name_factory.connect_setup(|_, li| {
            let listitem = li.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::builder().halign(gtk::Align::Start).build();
            listitem.set_child(Some(&label));
            listitem
                .property_expression("item")
                .chain_property::<DemoObject>("name")
                .bind(&label, "label", gtk::Widget::NONE);
        });
        widgets.demo_list.append_column(
            &gtk::ColumnViewColumn::builder()
                .title("Name")
                .resizable(true)
                .factory(&name_factory)
                .expand(true)
                .sorter(&gtk::StringSorter::new(Some(
                    &gtk::PropertyExpression::new(
                        DemoObject::static_type(),
                        None::<gtk::Expression>,
                        "name",
                    ),
                )))
                .build(),
        );

        let map_factory = gtk::SignalListItemFactory::new();
        map_factory.connect_setup(|_, li| {
            let listitem = li.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::builder().halign(gtk::Align::Start).build();
            listitem.set_child(Some(&label));
            listitem
                .property_expression("item")
                .chain_property::<DemoObject>("map")
                .bind(&label, "label", gtk::Widget::NONE);
        });
        widgets.demo_list.append_column(
            &gtk::ColumnViewColumn::builder()
                .title("Map")
                .resizable(true)
                .factory(&map_factory)
                .expand(true)
                .sorter(&gtk::StringSorter::new(Some(
                    &gtk::PropertyExpression::new(
                        DemoObject::static_type(),
                        None::<gtk::Expression>,
                        "map",
                    ),
                )))
                .build(),
        );

        let username_factory = gtk::SignalListItemFactory::new();
        username_factory.connect_setup(|_, li| {
            let listitem = li.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::builder().halign(gtk::Align::Start).build();
            listitem.set_child(Some(&label));
            listitem
                .property_expression("item")
                .chain_property::<DemoObject>("username")
                .bind(&label, "label", gtk::Widget::NONE);
        });
        widgets.demo_list.append_column(
            &gtk::ColumnViewColumn::builder()
                .title("Username")
                .resizable(true)
                .factory(&username_factory)
                .expand(true)
                .sorter(&gtk::StringSorter::new(Some(
                    &gtk::PropertyExpression::new(
                        DemoObject::static_type(),
                        None::<gtk::Expression>,
                        "username",
                    ),
                )))
                .build(),
        );

        let duration_factory = gtk::SignalListItemFactory::new();
        duration_factory.connect_setup(|_, li| {
            let listitem = li.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::builder().halign(gtk::Align::End).build();
            listitem.set_child(Some(&label));
            listitem
                .property_expression("item")
                .chain_property::<DemoObject>("duration")
                .chain_closure_with_callback(|v| {
                    humantime::format_duration(std::time::Duration::from_secs(
                        v[1].get::<f32>().unwrap() as u64,
                    ))
                    .to_string()
                })
                .bind(&label, "label", gtk::Widget::NONE);
        });
        widgets.demo_list.append_column(
            &gtk::ColumnViewColumn::builder()
                .title("Duration")
                .resizable(true)
                .factory(&duration_factory)
                .expand(true)
                .sorter(&gtk::NumericSorter::new(Some(
                    &gtk::PropertyExpression::new(
                        DemoObject::static_type(),
                        None::<gtk::Expression>,
                        "duration",
                    ),
                )))
                .build(),
        );

        let date_factory = gtk::SignalListItemFactory::new();
        date_factory.connect_setup(|_, li| {
            let listitem = li.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::builder().halign(gtk::Align::End).build();
            listitem.set_child(Some(&label));
            listitem
                .property_expression("item")
                .chain_property::<DemoObject>("created")
                .chain_closure_with_callback(|v| {
                    chrono::Local
                        .timestamp_millis_opt(v[1].get().unwrap())
                        .unwrap()
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                })
                .bind(&label, "label", gtk::Widget::NONE);
        });
        let date_column = &gtk::ColumnViewColumn::builder()
            .title("Created")
            .resizable(true)
            .factory(&date_factory)
            .expand(true)
            .sorter(&gtk::NumericSorter::new(Some(
                &gtk::PropertyExpression::new(
                    DemoObject::static_type(),
                    None::<gtk::Expression>,
                    "created",
                ),
            )))
            .build();

        widgets.demo_list.append_column(date_column);

        let size_factory = gtk::SignalListItemFactory::new();
        size_factory.connect_setup(|_, li| {
            let listitem = li.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::builder().halign(gtk::Align::End).build();
            listitem.set_child(Some(&label));
            listitem
                .property_expression("item")
                .chain_property::<DemoObject>("size")
                .chain_closure_with_callback(|v| {
                    format!(
                        "{:.2}B",
                        size_format::SizeFormatterBinary::new(v[1].get::<u64>().unwrap())
                    )
                })
                .bind(&label, "label", gtk::Widget::NONE);
        });
        widgets.demo_list.append_column(
            &gtk::ColumnViewColumn::builder()
                .title("Size")
                .resizable(true)
                .factory(&size_factory)
                .expand(true)
                .sorter(&gtk::NumericSorter::new(Some(
                    &gtk::PropertyExpression::new(
                        DemoObject::static_type(),
                        None::<gtk::Expression>,
                        "size",
                    ),
                )))
                .build(),
        );

        let bookmark_factory = gtk::SignalListItemFactory::new();
        bookmark_factory.connect_setup(|_, li| {
            let listitem = li.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::builder().halign(gtk::Align::End).build();
            listitem.set_child(Some(&label));
            listitem
                .property_expression("item")
                .chain_property::<DemoObject>("bookmarks")
                .bind(&label, "label", gtk::Widget::NONE);
        });
        widgets.demo_list.append_column(
            &gtk::ColumnViewColumn::builder()
                .title("Bookmarks")
                .resizable(true)
                .factory(&bookmark_factory)
                .expand(true)
                .sorter(&gtk::NumericSorter::new(Some(
                    &gtk::PropertyExpression::new(
                        DemoObject::static_type(),
                        None::<gtk::Expression>,
                        "bookmarks",
                    ),
                )))
                .build(),
        );

        widgets
            .demo_list
            .sort_by_column(Some(date_column), gtk::SortType::Descending);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            DemoListMsg::Update(demos, scroll) => {
                let model_set: HashSet<(String, u64), RandomState> =
                    HashSet::from_iter(self.list_model.into_iter().map(|d| {
                        let demo = d.unwrap().downcast::<DemoObject>().unwrap();
                        (demo.name(), demo.size())
                    }));

                let data_set: HashSet<(String, u64), RandomState> =
                    HashSet::from_iter(demos.iter().map(|t| {
                        let demo = t.1;
                        (
                            demo.filename.to_owned(),
                            demo.metadata.as_ref().unwrap().len(),
                        )
                    }));

                self.list_model.retain(|d| {
                    let d = d.downcast_ref::<DemoObject>().unwrap();
                    data_set.contains(&(d.name(), d.size()))
                });

                let added = data_set.difference(&model_set);
                for dn in added {
                    self.list_model
                        .append(&DemoObject::new(demos.get(&dn.0).unwrap()));
                }

                if scroll {
                    root.vadjustment().set_value(0.0);
                }
            }
            DemoListMsg::SelectionChanged => {
                let selected = self.list_selection.selection();
                if selected.is_empty() {
                    let _ = sender.output(DemoListOut::SelectionChanged(None));
                    return;
                }

                let model = self.list_selection.model().unwrap();
                let dem_name = model
                    .item(selected.nth(0))
                    .and_downcast_ref::<DemoObject>()
                    .unwrap()
                    .name();

                let _ = sender.output(DemoListOut::SelectionChanged(Some(dem_name)));
            }
        }
    }
}
