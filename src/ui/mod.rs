use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::RandomState;
use std::rc::Rc;

use gtk::gio::{ApplicationFlags, ListStore, Menu, SimpleAction};
use gtk::glib::clone;
use gtk::{glib, prelude::*, Adjustment, AlertDialog, Application, ApplicationWindow, Box, Button, CenterBox, FileDialog, Grid, HeaderBar, Label, MenuButton, MultiSelection, Paned, PopoverMenu, Scale, Separator, SortListModel};

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
        let demos = demos.borrow();
        let sel_model = selection.model().unwrap();
        let sort_model = sel_model.downcast_ref::<SortListModel>().unwrap().model().unwrap();
        let demo_model = sort_model.downcast_ref::<ListStore>().unwrap();

        let model_set: HashSet<String, RandomState> = HashSet::from_iter(demo_model.into_iter().map(|d|d.unwrap().downcast::<DemoObject>().unwrap().name()));
        let data_set: HashSet<String, RandomState> = HashSet::from_iter(demos.get_demos().into_iter().map(|t|t.0.to_owned()));
        
        demo_model.retain(|d|{
            let d = d.downcast_ref::<DemoObject>().unwrap().name();
            data_set.contains(&d)
        });

        let added = data_set.difference(&model_set);
        for dn in added {
            demo_model.append(&DemoObject::new(demos.get_demo(&dn).unwrap()));
        }
    }));

    update_demos();

    let bottom_box = Box::builder().orientation(gtk::Orientation::Vertical).vexpand(true).hexpand(true).build();
    let grid = Grid::builder().column_homogeneous(false).margin_end(5).margin_start(5).margin_bottom(5).build();
    bottom_box.append(&grid);

    let playhead = Scale::builder().orientation(gtk::Orientation::Horizontal).hexpand(true).build();
    playhead.set_range(0.0, 100.0);
    playhead.set_adjustment(&Adjustment::builder().step_increment(1.0).build());
    grid.attach(&playhead, 0, 0, 1, 1);

    let timestamp_label = Label::builder().halign(gtk::Align::Center).valign(gtk::Align::Start).justify(gtk::Justification::Center).selectable(true).label("00:00.00\ntick 0").margin_top(10).margin_bottom(10).build();
    grid.attach(&timestamp_label, 1, 0, 1, 1);

    playhead.connect_value_changed(clone!(@weak timestamp_label, @weak demos, @weak selection => move |ph|{
        let tps = get_selected_demo(&selection, &demos.borrow()).unwrap().tps().unwrap_or(Demo::TICKRATE);
        let secs = ticks_to_sec(ph.value() as u32, tps);
        timestamp_label.set_label(format!("{}\ntick {}", sec_to_timestamp(secs).as_str(), ph.value() as u32).as_str());
    }));



    let (detail_view, update_detail_view) = build_detail_view(clone!(@weak playhead => move |t|{
        playhead.set_value(t as f64);
    }));
    bottom_box.append(&detail_view);



    let left_button_box = Box::builder().orientation(gtk::Orientation::Horizontal).spacing(5).build();
    grid.attach(&CenterBox::builder().start_widget(&left_button_box).build(), 0, 1, 2, 1);

    let play_button = Button::builder().icon_name("media-playback-start-symbolic").tooltip_text("Play demo").build();
    play_button.connect_clicked(clone!(@weak demos, @weak selection, @weak rcon => move |b| {
        glib::spawn_future_local(clone!(@weak demos, @weak selection, @weak rcon, @weak b => async move {
            b.set_sensitive(false);
            let b_demos = demos.borrow();
            let selected = get_selected_demo(&selection, &b_demos).unwrap();
            let _ = rcon.borrow_mut().play_demo(selected).await;
            b.set_sensitive(true);
        }));
    }));
    left_button_box.append(&play_button);

    let seek_button = Button::builder().icon_name("find-location-symbolic").tooltip_text("Skip to tick").build();
    left_button_box.append(&seek_button);

    seek_button.connect_clicked(clone!(@weak rcon, @weak playhead => move |b|{
        glib::spawn_future_local(clone!(@weak rcon, @weak playhead, @weak b => async move {
            b.set_sensitive(false);
            let _ = rcon.borrow_mut().skip_to_tick(playhead.value() as u32, false).await;
            b.set_sensitive(true);
        }));
    }));
    

    let stop_playback_button = Button::builder().icon_name("media-playback-stop-symbolic").tooltip_text("Stop Playback").build();
    stop_playback_button.connect_clicked(clone!(@weak rcon => move |b|{
        glib::spawn_future_local(clone!(@weak rcon, @weak b => async move {
            b.set_sensitive(false);
            let _ = rcon.borrow_mut().stop_playback().await;
            b.set_sensitive(true);
        }));
    }));
    left_button_box.append(&stop_playback_button);

    left_button_box.append(&Separator::builder().orientation(gtk::Orientation::Vertical).build());

    let skip_backward_button = Button::builder().icon_name("media-seek-backward-symbolic").tooltip_text("-30s").build();

    skip_backward_button.connect_clicked(clone!(@weak playhead, @weak selection, @weak demos => move |_|{
        let tps = get_selected_demo(&selection, &demos.borrow()).unwrap().tps().unwrap_or(Demo::TICKRATE);
        playhead.set_value(playhead.value() - 30.0*tps as f64);
    }));
    left_button_box.append(&skip_backward_button);

    let skip_forward_button = Button::builder().icon_name("media-seek-forward-symbolic").tooltip_text("+30s").build();

    skip_forward_button.connect_clicked(clone!(@weak playhead, @weak selection, @weak demos => move |_|{
        let tps = get_selected_demo(&selection, &demos.borrow()).unwrap().tps().unwrap_or(Demo::TICKRATE);
        playhead.set_value(playhead.value() + 30.0*tps as f64);
    }));
    left_button_box.append(&skip_forward_button);


    selection.connect_selection_changed(clone!(@strong update_detail_view, @weak demos, @weak playhead => move|s,_,_|{
        let demos = demos.borrow();
        let demo = get_selected_demo(s, &demos);
        if demo.is_none(){
            playhead.set_value(0.0);
            playhead.set_range(0.0, 1.0);
            playhead.clear_marks();
            playhead.set_sensitive(false);
            update_detail_view(None);

            left_button_box.set_sensitive(false);
            return;
        }
        let demo = demo.unwrap();
        left_button_box.set_sensitive(true);
        playhead.set_sensitive(true);
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
    
    let folderbutton = Button::builder().icon_name("folder-symbolic").tooltip_text("Select demo folder").width_request(20).height_request(20).build();
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
    let menubutton = MenuButton::builder().icon_name("open-menu-symbolic").popover(&menu).build();
    
    titlebar.pack_end(&menubutton);


    let delete_button = Button::builder().icon_name("edit-delete-symbolic").tooltip_text("Delete selected demo").build();

    delete_button.connect_clicked(clone!(@weak demos, @weak selection, @strong update_demos, @weak window => move|_|{
        glib::spawn_future_local(clone!(@weak demos, @weak selection, @weak update_demos, @weak window => async move {

            {
                let mut demos = demos.borrow_mut();
                let sel_demos = get_all_selected_demos(&selection);
                let ad = AlertDialog::builder().buttons(vec!["Delete", "Cancel"]).default_button(1).cancel_button(1).detail("Deleting selected demos!").message("Are you sure?").modal(true).build();
                match ad.choose_future(Some(&window)).await {
                    Ok(choice) => match choice {0 => {}, _ => return},
                    Err(e) => {log::warn!("Dialog error? {}", e); return;}
                };

                for d in sel_demos {
                    demos.delete_demo(&d).await;
                }
                
            }
            update_demos();
        }));
    }));

    titlebar.pack_end(&delete_button);


    
    let pane = Paned::builder().orientation(gtk::Orientation::Vertical).start_child(&demo_scroll).end_child(&bottom_box).build();
    
    window.set_child(Some(&pane));
    window.set_titlebar(Some(&titlebar));
    window.present();
}

fn get_selected_demo<'a>(selection: &MultiSelection, demos: &'a DemoManager) -> Option<&'a Demo>{
    let selected = selection.selection();
    if selected.is_empty() {
        return None;
    }
    let model = selection.model().unwrap();
    let dem_name = model.item(selected.nth(0)).and_downcast_ref::<DemoObject>().unwrap().name();
    Some(demos.get_demo(&dem_name).unwrap())
}

fn get_all_selected_demos(selection: &MultiSelection) -> Vec<String> {
    let selected = selection.selection();
    if selected.is_empty() {
        return vec![];
    }

    let model = selection.model().unwrap();
    
    (0..selected.size() as u32).map(|i|{
        model.item(selected.nth(i)).and_downcast_ref::<DemoObject>().unwrap().name()
    }).collect()
}