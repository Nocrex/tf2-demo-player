use std::cell::RefCell;
use std::rc::Rc;

use gtk::gio::{ApplicationFlags, ListStore, Menu, SimpleAction};
use gtk::glib::clone;
use gtk::{glib, prelude::*, Adjustment, AlertDialog, Application, ApplicationWindow, Box, Button, FileDialog, Grid, HeaderBar, Label, MenuButton, Paned, PopoverMenu, Scale, SelectionModel, SingleSelection, SortListModel};

use crate::demo_manager::{Demo, DemoManager};
use crate::rcon_manager::RconManager;
use crate::settings::Settings;
use crate::util::{sec_to_timestamp, ticks_to_sec};

mod demo_object;
use demo_object::DemoObject;

mod demo_list;
use demo_list::build_demo_list;

mod detail_view;
use detail_view::build_detail_view;

mod event_object;

pub struct UI {
    app: Application,
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
            build_ui(rcon.clone(), demos.clone(), settings.clone(), app);
        }));
        ui
    }

    pub fn run(&self) {
        self.app.run();
    }
}

fn build_ui(rcon: Rc<RefCell<RconManager>>, demos: Rc<RefCell<DemoManager>>, settings: Rc<RefCell<Settings>>, app: &Application){
    let window = ApplicationWindow::builder()
        .application(app)
        .width_request(1000)
        .height_request(800)
        .build();




    let (demo_scroll, selection) = build_demo_list();

    let update_demos = Rc::new(clone!(@weak selection, @weak demos => move || {
        let sel_model = selection.model().unwrap();
        let sort_model = sel_model.downcast_ref::<SortListModel>().unwrap().model().unwrap();
        let demo_model = sort_model.downcast_ref::<ListStore>().unwrap();
        demo_model.remove_all();

        for demo in demos.borrow().get_demos() {
            demo_model.append(&DemoObject::new(demo));
        }
    }));

    update_demos();


    fn get_selected_demo<'a>(selection: &SingleSelection, demos: &'a DemoManager) -> &'a Demo{
        let dem_name = selection.selected_item().unwrap().downcast_ref::<DemoObject>().unwrap().name();
        demos.get_demos().iter().find(|d|d.filename == dem_name).unwrap()
    }


    let grid = Grid::builder().column_homogeneous(false).margin_end(5).height_request(250).build();

    let playhead = Scale::builder().orientation(gtk::Orientation::Horizontal).hexpand(true).build();
    playhead.set_range(0.0, 100.0);
    playhead.set_adjustment(&Adjustment::builder().step_increment(1.0).build());
    grid.attach(&playhead, 0, 0, 1, 1);

    let timestamp_label = Label::builder().halign(gtk::Align::Center).valign(gtk::Align::Start).justify(gtk::Justification::Center).selectable(true).label("00:00.00\ntick 0").margin_top(10).margin_bottom(10).build();
    grid.attach(&timestamp_label, 1, 0, 1, 1);

    playhead.connect_value_changed(clone!(@weak timestamp_label, @weak demos, @weak selection => move |ph|{
        let tps = get_selected_demo(&selection, &demos.borrow()).header.as_ref().map_or(66.667, |h|h.ticks as f32/h.duration);
        let secs = ticks_to_sec(ph.value() as u32, tps);
        timestamp_label.set_label(format!("{}\ntick {}", sec_to_timestamp(secs).as_str(), ph.value() as u32).as_str());
    }));



    let (detail_view, update_detail_view) = build_detail_view(clone!(@weak playhead => move |t|{
        playhead.set_value(t as f64);
    }));
    grid.attach(&detail_view, 0, 1, 1, 1);



    let button_box = Box::builder().orientation(gtk::Orientation::Vertical).spacing(5).width_request(100).margin_start(5).build();
    grid.attach(&button_box, 1, 1, 1, 1);

    let play_button = Button::builder().label("Play").build();
    play_button.connect_clicked(clone!(@weak demos, @weak selection, @weak rcon => move |b| {
        glib::spawn_future_local(clone!(@weak demos, @weak selection, @weak rcon, @weak b => async move {
            b.set_sensitive(false);
            let b_demos = demos.borrow();
            let selected = get_selected_demo(&selection, &b_demos);
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
                let demo = get_selected_demo(&selection, &demos).clone();
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

    seek_button.connect_clicked(clone!(@weak rcon, @weak playhead => move |b|{
        glib::spawn_future_local(clone!(@weak rcon, @weak playhead, @weak b => async move {
            b.set_sensitive(false);
            let _ = rcon.borrow_mut().skip_to_tick(playhead.value() as u32, true).await;
            b.set_sensitive(true);
        }));
    }));




    selection.connect_selection_changed(clone!(@strong update_detail_view, @weak demos, @weak playhead => move|s,_,_|{
        let demos = demos.borrow();
        if demos.get_demos().is_empty(){
            playhead.set_value(0.0);
            playhead.set_range(0.0, 1.0);
            playhead.clear_marks();
            playhead.set_sensitive(false);
            update_detail_view(None);

            button_box.set_sensitive(false);
            return;
        }
        button_box.set_sensitive(true);
        playhead.set_sensitive(true);
        let demo = get_selected_demo(s, &demos);
        update_detail_view(Some(demo.to_owned()));
        playhead.set_value(0.0);
        playhead.clear_marks();
        playhead.set_range(0.0, demo.header.as_ref().map_or(0, |h|h.ticks) as f64);
        for event in &demo.events {
            playhead.add_mark(event.tick as f64, gtk::PositionType::Bottom, None);
        }
    }));

    selection.emit_by_name::<()>("selection-changed", &[&0u32.to_value(),&0u32.to_value()]);



    
    let titlebar = HeaderBar::new();
    titlebar.set_title_widget(Some(&Label::builder().label("Demo Player").build()));
    
    let folderbutton = Button::builder().icon_name("folder-open").tooltip_text("Select demo folder").width_request(20).height_request(20).build();
    titlebar.pack_start(&folderbutton);
    
    folderbutton.connect_clicked(clone!(@weak update_demos, @weak demos, @weak window, @weak settings, @weak selection => move|_|{
        glib::spawn_future_local(clone!(@weak update_demos, @weak demos, @weak window, @weak settings, @weak selection => async move {
            let dia = FileDialog::builder().build();
            let res = dia.select_folder_future(Some(&window)).await;
            if let Ok(file) = res{
                settings.borrow_mut().demo_folder_path = file.path().unwrap().display().to_string();
                settings.borrow().save();
                demos.borrow_mut().load_demos(&settings.borrow().demo_folder_path).await;
                update_demos();
                selection.emit_by_name::<()>("selection-changed", &[&0u32.to_value(),&0u32.to_value()]);
            }
        }));
    }));


    let menu_model = Menu::new();
    menu_model.append(Some("Reload Folder"), Some("app.reload"));
    menu_model.append(Some("Delete unfinished demos"), Some("app.clean-unfinished"));
    menu_model.append(Some("Delete demos without bookmarks"), Some("app.clean-unmarked"));
    menu_model.freeze();

    let reload_action = SimpleAction::new("reload", None);
    app.add_action(&reload_action);
    reload_action.connect_activate(clone!(@weak demos, @weak settings, @weak update_demos => move |_,_|{
        glib::spawn_future_local(clone!(@weak demos, @weak settings, @weak update_demos => async move {
            demos.borrow_mut().load_demos(&settings.borrow().demo_folder_path).await;
            update_demos();
        }));
    }));
    
    let clean_unfinished_action = SimpleAction::new("clean-unfinished", None);
    app.add_action(&clean_unfinished_action);
    clean_unfinished_action.connect_activate(|_,_|{
        todo!();
    });
    
    let clean_unmarked_action = SimpleAction::new("clean-unmarked", None);
    app.add_action(&clean_unmarked_action);
    clean_unmarked_action.connect_activate(|_,_|{
        todo!();
    });

    let menu = PopoverMenu::from_model(Some(&menu_model));
    let menubutton = MenuButton::builder().icon_name("application-menu").popover(&menu).build();
    
    titlebar.pack_end(&menubutton);



    
    let pane = Paned::builder().orientation(gtk::Orientation::Vertical).start_child(&demo_scroll).end_child(&grid).build();
    
    window.set_child(Some(&pane));
    window.set_titlebar(Some(&titlebar));
    window.present();
}