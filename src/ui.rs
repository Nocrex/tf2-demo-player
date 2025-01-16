use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::gio::{ApplicationFlags, ListStore};
use gtk::glib::{Object, clone};
use gtk::{glib, prelude::*, Adjustment, AlertDialog, Application, ApplicationWindow, Box, Button, ColumnView, ColumnViewColumn, FileDialog, Grid, HeaderBar, Label, ListItem, Notebook, Paned, Scale, ScrolledWindow, SignalListItemFactory, SingleSelection};

use crate::demo_manager::{Demo, DemoManager};
use crate::rcon_manager::RconManager;
use crate::settings::Settings;
use crate::util::{sec_to_timestamp, ticks_to_sec};

pub struct UI {
    app: Application,
}

glib::wrapper!{
    pub struct DemoObject(ObjectSubclass<imp::DemoObject>);
}

impl DemoObject {
    pub fn new(demo: &Demo) -> Self {
        let mut b = Object::builder()
            .property("name", demo.filename.to_owned())
            .property("bookmarks", demo.events.len() as u32);

        if let Some(header) = &demo.header{
            b = b.property("map", header.map.to_owned())
                .property("username", header.nick.to_owned())
                .property("duration", humantime::format_duration(Duration::from_secs(header.duration as u64)).to_string());
        }

        b.build()
    }
}

mod imp {
    use std::cell::RefCell;
    use std::cell::Cell;

    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::DemoObject)]
    pub struct DemoObject {
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        map: RefCell<String>,
        #[property(get, set)]
        username: RefCell<String>,
        #[property(get, set)]
        duration: RefCell<String>,
        #[property(get, set)]
        bookmarks: Cell<u32>
    }


    #[glib::object_subclass]
    impl ObjectSubclass for DemoObject {
        const NAME: &'static str = "DemoObject";
        type Type = super::DemoObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for DemoObject {}
}

impl UI {
    pub fn new(rcon: RconManager, demos: DemoManager, settings: Settings) -> UI {
        let ui = UI {
            app: Application::new(None::<String>, ApplicationFlags::empty()),
        };
        let rcon = Rc::new(RefCell::new(rcon));
        let demos = Rc::new(RefCell::new(demos));
        let settings = Rc::new(RefCell::new(settings));

        ui.app.connect_activate(clone!(@strong rcon, @strong demos, @strong settings => move |app|{

            let window = ApplicationWindow::builder()
                .application(app)
                .width_request(800)
                .height_request(600)
                .build();

            let (demo_scroll, selection) = Self::build_demo_list();

            let update_demos = Rc::new(clone!(@weak demo_scroll, @weak demos => move || {
                let cv = demo_scroll.child().and_downcast::<ColumnView>().unwrap();
                let sel_model = cv.model().unwrap().downcast::<SingleSelection>().unwrap().model().unwrap();
                let demo_model = sel_model.downcast_ref::<ListStore>().unwrap();
                demo_model.remove_all();

                for demo in demos.borrow().get_demos() {
                    demo_model.append(&DemoObject::new(demo));
                }
            }));

            update_demos();

            let grid = Grid::builder().column_homogeneous(false).margin_end(15).build();

            let playhead = Scale::builder().orientation(gtk::Orientation::Horizontal).hexpand(true).build();
            playhead.set_range(0.0, 100.0);
            playhead.set_adjustment(&Adjustment::builder().step_increment(1.0).build());
            grid.attach(&playhead, 0, 0, 1, 1);

            let timestamp_label = Label::builder().halign(gtk::Align::Center).valign(gtk::Align::Start).justify(gtk::Justification::Center).selectable(true).label("00:00\ntick 0").margin_top(10).build();
            grid.attach(&timestamp_label, 1, 0, 1, 1);

            playhead.connect_value_changed(clone!(@weak timestamp_label => move |ph|{
                let secs = ticks_to_sec(ph.value() as u32);
                timestamp_label.set_label(format!("{}\ntick {}", sec_to_timestamp(secs).as_str(), ph.value() as u32).as_str());
            }));

            let deets = Label::new(None);
            deets.set_halign(gtk::Align::Start);
            let deets_w = ScrolledWindow::builder().child(&deets).hexpand(true).vexpand(true).build();

            let detail_tabs = Notebook::builder().show_border(false).build();
            
            detail_tabs.append_page(&deets_w, Some(&Label::new(Some(&"Details"))));
            
            detail_tabs.append_page(&Label::new(Some(&"test")), Some(&Label::new(Some(&"Bookmarks"))));

            grid.attach(&detail_tabs, 0, 1, 1, 1);
            
            let button_box = Box::builder().orientation(gtk::Orientation::Vertical).spacing(5).width_request(100).build();
            grid.attach(&button_box, 1, 1, 1, 1);

            let play_button = Button::builder().label("Play").build();
            play_button.connect_clicked(clone!(@weak demos, @weak selection, @weak rcon => move |b| {
                glib::spawn_future_local(clone!(@weak demos, @weak selection, @weak rcon, @weak b => async move {
                    b.set_sensitive(false);
                    let b_demos = demos.borrow();
                    let selected = b_demos.get_demos().get(selection.selected() as usize).unwrap();
                    let _ = rcon.borrow_mut().play_demo(selected).await;
                    b.set_sensitive(true);
                }));
            }));
            button_box.append(&play_button);

            let delete_button = Button::builder().label("Delete").build();
            button_box.append(&delete_button);

            delete_button.connect_clicked(clone!(@weak demos, @weak selection, @strong update_demos, @weak window => move|_|{
                glib::spawn_future_local(clone!(@weak demos, @weak selection, @weak update_demos, @weak window => async move {

                    {
                        let mut demos = demos.borrow_mut();
                        let demo = demos.get_demos().get(selection.selected() as usize).unwrap().clone();
                        let ad = AlertDialog::builder().buttons(vec!["Delete", "Cancel"]).default_button(1).cancel_button(1).message(format!("Deleting {}", demo.filename).as_str()).message("Are you sure?").modal(true).build();
                        match ad.choose_future(Some(&window)).await {
                            Ok(choice) => match choice {0 => {}, _ => return},
                            Err(e) => log::warn!("Dialog error? {}", e)
                        };

                        demos.delete_demo(&demo).await;
                        
                    }
                    update_demos();
                }));
            }));

            let seek_button = Button::builder().label("Seek").build();
            button_box.append(&seek_button);

            seek_button.connect_clicked(clone!(@weak rcon, @weak playhead => move |_|{
                glib::spawn_future_local(clone!(@weak rcon, @weak playhead => async move {
                    let _ = rcon.borrow_mut().skip_to_tick(playhead.value() as u32).await;
                }));
            }));

            selection.connect_selection_changed(clone!(@weak deets, @weak demos, @weak playhead => move|s,_,_|{
                let demos = demos.borrow();
                if demos.get_demos().is_empty(){
                    playhead.set_value(0.0);
                    playhead.set_range(0.0, 1.0);
                    playhead.clear_marks();
                    playhead.set_sensitive(false);
                    deets.set_label("");

                    button_box.set_sensitive(false);
                    return;
                }
                button_box.set_sensitive(true);
                playhead.set_sensitive(true);
                let demo = demos.get_demos().get(s.selected() as usize).unwrap();
                deets.set_label(format!("{:#?}", demos.get_demos().get(s.selected() as usize).unwrap()).as_str());
                playhead.set_value(0.0);
                playhead.clear_marks();
                playhead.set_range(0.0, demo.header.as_ref().unwrap().ticks as f64);
                for event in &demo.events {
                    playhead.add_mark(event.tick as f64, gtk::PositionType::Bottom, Some(demo.events.iter().position(|e|e.tick == event.tick).unwrap().to_string().as_str()));
                }
            }));

            selection.emit_by_name::<()>("selection-changed", &[&0u32.to_value(),&0u32.to_value()]);

            let pane = Paned::builder().orientation(gtk::Orientation::Vertical).start_child(&demo_scroll).end_child(&grid).build();

            let titlebar = HeaderBar::new();
            titlebar.set_title_widget(Some(&Label::builder().label("Demo Player").build()));

            let folderbutton = Button::builder().icon_name("folder-open").tooltip_text("Select demo folder").width_request(20).height_request(20).build();
            titlebar.pack_start(&folderbutton);

            folderbutton.connect_clicked(clone!(@weak update_demos, @weak demos, @weak window, @weak settings => move|_|{
                glib::spawn_future_local(clone!(@weak update_demos, @weak demos, @weak window, @weak settings => async move {
                    let dia = FileDialog::builder().build();
                    let res = dia.select_folder_future(Some(&window)).await;
                    if let Ok(file) = res{
                        settings.borrow_mut().demo_folder_path = file.path().unwrap().display().to_string();
                        settings.borrow().save();
                        demos.borrow_mut().load_demos(&settings.borrow().demo_folder_path).await;
                        update_demos();
                    }

                }));
            }));

            window.set_child(Some(&pane));
            window.set_titlebar(Some(&titlebar));
            window.present();
        }));
        ui
    }

    fn build_demo_list() -> (ScrolledWindow, SingleSelection){
        let demo_model = ListStore::new::<Object>();

        let selection = SingleSelection::builder().model(&demo_model).build();

        let demo_list = ColumnView::builder()
            .vexpand(true)
            .hexpand(true)
            .model(&selection).build();

        let name_factory = SignalListItemFactory::new();
        name_factory.connect_bind(|_, li|{
            let li = li.downcast_ref::<ListItem>().unwrap();
            let name: String = li.item().and_downcast_ref::<DemoObject>().unwrap().property("name");
            li.child().and_downcast_ref::<Label>().unwrap().set_label(&name);
        });
        name_factory.connect_setup(|_, li|{
            li.downcast_ref::<ListItem>().unwrap().set_child(Some(&Label::builder().halign(gtk::Align::Start).build()));
        });
        
        demo_list.append_column(&ColumnViewColumn::builder().title("Name").resizable(true).factory(&name_factory).expand(true).build());

        let map_factory = SignalListItemFactory::new();
        map_factory.connect_bind(|_, li|{
            let li = li.downcast_ref::<ListItem>().unwrap();
            let map: String = li.item().and_downcast_ref::<DemoObject>().unwrap().property("map");
            li.child().and_downcast_ref::<Label>().unwrap().set_label(&map);
        });
        map_factory.connect_setup(|_, li|{
            li.downcast_ref::<ListItem>().unwrap().set_child(Some(&Label::builder().halign(gtk::Align::Start).build()));
        });
        demo_list.append_column(&ColumnViewColumn::builder().title("Map").resizable(true).factory(&map_factory).expand(true).build());

        let duration_factory = SignalListItemFactory::new();
        duration_factory.connect_bind(|_, li|{
            let li = li.downcast_ref::<ListItem>().unwrap();
            let duration: String = li.item().and_downcast_ref::<DemoObject>().unwrap().property("duration");
            li.child().and_downcast_ref::<Label>().unwrap().set_label(&duration);
        });
        duration_factory.connect_setup(|_, li|{
            li.downcast_ref::<ListItem>().unwrap().set_child(Some(&Label::builder().halign(gtk::Align::Start).build()));
        });

        demo_list.append_column(&ColumnViewColumn::builder().title("Duration").resizable(true).factory(&duration_factory).expand(true).build());

        let username_factory = SignalListItemFactory::new();
        username_factory.connect_bind(|_, li|{
            let li = li.downcast_ref::<ListItem>().unwrap();
            let username: String = li.item().and_downcast_ref::<DemoObject>().unwrap().property("username");
            li.child().and_downcast_ref::<Label>().unwrap().set_label(&username);
        });
        username_factory.connect_setup(|_, li|{
            li.downcast_ref::<ListItem>().unwrap().set_child(Some(&Label::builder().halign(gtk::Align::Start).build()));
        });

        demo_list.append_column(&ColumnViewColumn::builder().title("Username").resizable(true).factory(&username_factory).expand(true).build());

        let bookmark_factory = SignalListItemFactory::new();
        bookmark_factory.connect_bind(|_, li|{
            let li = li.downcast_ref::<ListItem>().unwrap();
            let bookmarks: u32 = li.item().and_downcast_ref::<DemoObject>().unwrap().property("bookmarks");
            li.child().and_downcast_ref::<Label>().unwrap().set_label(format!("{}",bookmarks).as_str());
        });
        bookmark_factory.connect_setup(|_, li|{
            li.downcast_ref::<ListItem>().unwrap().set_child(Some(&Label::builder().halign(gtk::Align::Start).build()));
        });
        demo_list.append_column(&ColumnViewColumn::builder().title("# Bookmarks").resizable(true).factory(&bookmark_factory).expand(true).build());

        let demo_scroll = ScrolledWindow::builder().has_frame(true).hscrollbar_policy(gtk::PolicyType::Never).child(&demo_list).height_request(150).build();

        (demo_scroll, selection)
    }

    pub fn run(&self) {
        self.app.run();
    }
}