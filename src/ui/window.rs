use std::{cell::RefCell, collections::HashSet, hash::RandomState, rc::Rc};

use glib::Object;
use gtk::{gio::{self, SimpleAction}, glib::{self, clone, subclass::types::ObjectSubclassIsExt}, prelude::*, Button, CenterBox, ColumnViewColumn, FileDialog, MultiSelection, NumericSorter, SingleSelection, SortListModel};
use adw::{prelude::*, Application};

use crate::{demo_manager::{Demo, DemoManager, Event}, rcon_manager::RconManager, settings::Settings, util::{sec_to_timestamp, ticks_to_sec}};

use super::{demo_object::DemoObject, event_dialog::EventDialog, event_object::EventObject, settings_window::SettingsWindow};

use std::time::Duration;

use chrono::TimeZone;
use gtk::{Expression, Label, ListItem, PropertyExpression, SignalListItemFactory, StringSorter, Widget};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, adw::Window, gtk::Window, gtk::ApplicationWindow, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

mod imp {
    use std::cell::RefCell;
    use std::rc::Rc;

    use adw::subclass::application_window::AdwApplicationWindowImpl;
    use glib::subclass::InitializingObject;
    use gtk::{gio, Box, Button, ColumnView, Entry, Label, ListView, MultiSelection, Paned, Scale, SingleSelection, TextView};
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::demo_manager::DemoManager;
    use crate::rcon_manager::RconManager;
    use crate::settings::Settings;
    
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/nocrex/tfdemoplayer/window.ui")]
    pub struct Window {
        pub demo_manager: RefCell<Option<Rc<RefCell<DemoManager>>>>,
        pub settings: RefCell<Option<Rc<RefCell<Settings>>>>,
        pub rcon_manager: RefCell<Option<Rc<RefCell<RconManager>>>>,

        #[template_child]
        pub button_open_folder: TemplateChild<Button>,
        #[template_child]
        pub delete_button: TemplateChild<Button>,
        #[template_child]
        pub reload_button: TemplateChild<Button>,

        #[template_child]
        pub demo_list: TemplateChild<ColumnView>,
        pub demo_model: RefCell<Option<gio::ListStore>>,
        pub demo_selection: RefCell<Option<MultiSelection>>,

        #[template_child]
        pub playbar: TemplateChild<Scale>,
        #[template_child]
        pub timestamp_label: TemplateChild<Label>,
        #[template_child]
        pub duration_label: TemplateChild<Label>,

        #[template_child]
        pub left_button_box: TemplateChild<Box>,
        #[template_child]
        pub play_button: TemplateChild<Button>,
        #[template_child]
        pub seek_button: TemplateChild<Button>,
        #[template_child]
        pub stop_playback_button: TemplateChild<Button>,
        #[template_child]
        pub skip_backward_button: TemplateChild<Button>,
        #[template_child]
        pub skip_forward_button: TemplateChild<Button>,
        #[template_child]
        pub convert_to_replay_button: TemplateChild<Button>,
        
        #[template_child]
        pub detail_edit_cancel: TemplateChild<Button>,
        #[template_child]
        pub detail_edit_save: TemplateChild<Button>,
        #[template_child]
        pub detail_edit: TemplateChild<Box>,

        #[template_child]
        pub detail_box: TemplateChild<Paned>,

        #[template_child]
        pub name_entry: TemplateChild<Entry>,
        #[template_child]
        pub map_entry: TemplateChild<Entry>,
        #[template_child]
        pub nick_entry: TemplateChild<Entry>,
        #[template_child]
        pub duration_entry: TemplateChild<Entry>,
        #[template_child]
        pub server_entry: TemplateChild<Entry>,
        #[template_child]
        pub notes_area: TemplateChild<TextView>,

        #[template_child]
        pub event_list: TemplateChild<ListView>,
        pub event_selection: RefCell<Option<SingleSelection>>,
        pub event_model: RefCell<Option<gio::ListStore>>,
        #[template_child]
        pub event_add: TemplateChild<Button>,
        #[template_child]
        pub event_remove: TemplateChild<Button>,
        #[template_child]
        pub event_edit: TemplateChild<Button>,
    }
    
    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "TFDemoPlayer";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;
    
        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }
    
        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_main_controls();
            obj.setup_titlebar_callbacks();
            obj.setup_demo_list();
            obj.setup_detail_view();
        }
    }
    
    impl WidgetImpl for Window {}
    
    impl WindowImpl for Window {}

    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

impl Window {
    pub fn new(app: &Application) -> Self {
        let obj: Window = Object::builder().property("application", app).build();
        unsafe{
            let demo_manager: DemoManager = app.steal_data::<DemoManager>("demo_manager").unwrap();
            let rcon_manager: RconManager = app.steal_data::<RconManager>("rcon_manager").unwrap();
            let settings: Settings = app.steal_data::<Settings>("settings").unwrap();
            obj.imp().demo_manager.replace(Some(Rc::new(RefCell::new(demo_manager))));
            obj.imp().rcon_manager.replace(Some(Rc::new(RefCell::new(rcon_manager))));
            obj.imp().settings.replace(Some(Rc::new(RefCell::new(settings))));
        }
        obj.refresh();
        obj.register_actions();
        obj.open_settings_on_first_launch();
        obj
    }

    fn register_actions(&self){
        let clean_unfinished_action = SimpleAction::new("clean-unfinished", None);
        self.application().unwrap().add_action(&clean_unfinished_action);
        clean_unfinished_action.connect_activate(clone!(@weak self as wnd => move|_,_|{
            glib::spawn_future_local(clone!(@weak wnd => async move {
                if !wnd.delete_dialog().await {
                    return;
                }
                wnd.demo_manager().borrow_mut().delete_empty_demos().await;
                wnd.refresh();
            }));
        }));
        
        let clean_unmarked_action = SimpleAction::new("clean-unmarked", None);
        self.application().unwrap().add_action(&clean_unmarked_action);
        clean_unmarked_action.connect_activate(clone!(@weak self as wnd => move|_,_|{
            glib::spawn_future_local(clone!(@weak wnd => async move {
                if !wnd.delete_dialog().await {
                    return;
                }
                wnd.demo_manager().borrow_mut().delete_unmarked_demos().await;
                wnd.refresh();
            }));
        }));

        let open_settings = SimpleAction::new("open-settings", None);
        self.application().unwrap().add_action(&open_settings);
        open_settings.connect_activate(clone!(@weak self as wnd => move |_,_|{
            SettingsWindow::new(&wnd).show();
        }));
    }

    pub fn refresh(&self){
        self.update_demos();
        self.selection().emit_by_name::<()>("selection-changed", &[&0u32.to_value(),&0u32.to_value()]);
    }
    
    pub fn demo_manager(&self) -> Rc<RefCell<DemoManager>> {
        self.imp().demo_manager.borrow().clone().unwrap()
    }

    pub fn settings(&self) -> Rc<RefCell<Settings>> {
        self.imp().settings.borrow().clone().unwrap()
    }
    
    pub fn rcon_manager(&self) -> Rc<RefCell<RconManager>> {
        self.imp().rcon_manager.borrow().clone().unwrap()
    }

    fn selection(&self) -> MultiSelection{
        self.imp().demo_selection.borrow().clone().unwrap()
    }

    fn event_model(&self) -> gio::ListStore {
        self.imp().event_model.borrow().clone().unwrap()
    }

    fn update_event_selection(&self) {
        self.imp().event_selection.borrow().clone().unwrap().emit_by_name::<()>("selection-changed", &[&0u32.to_value(),&0u32.to_value()]);
    }

    async fn delete_dialog(&self) -> bool {
        let ad = adw::AlertDialog::builder().default_response("cancel").close_response("cancel").body("Deleting selected demos!").heading("Are you sure?").build();
        ad.add_response("cancel", "Cancel");
        ad.add_response("delete", "Delete");
        ad.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        let res = ad.choose_future(self).await;
        match res.as_str() {
            "delete" => true,
            _ => false
        }
    }
    
    async fn notice_dialog(&self, title: &str, message: &str) {
        let ad = adw::AlertDialog::builder().default_response("ok").close_response("ok").body(message).heading(title).build();
        ad.add_response("ok", "OK");
        ad.choose_future(self).await;
    }

    fn get_selected_demo(&self) -> Option<Demo>{
        let selected = self.selection().selection();
        if selected.is_empty() {
            return None;
        }
        let model = self.selection().model().unwrap();
        let dem_name = model.item(selected.nth(0)).and_downcast_ref::<DemoObject>().unwrap().name();
        Some(self.demo_manager().borrow().get_demo(&dem_name).unwrap().clone()) //TODO: return reference somehow
    }

    fn get_current_tps(&self) -> f32 {
        return self.get_selected_demo().and_then(|d|d.tps()).map_or(Demo::TICKRATE, |t|t);
    }
    
    fn get_all_selected_demos(&self) -> Vec<String> {
        let selected = self.selection().selection();
        if selected.is_empty() {
            return vec![];
        }
    
        let model = self.selection().model().unwrap();
        
        (0..selected.size() as u32).map(|i|{
            model.item(selected.nth(i)).and_downcast_ref::<DemoObject>().unwrap().name()
        }).collect()
    }

    fn update_demos(&self){
        let demo_manager = self.demo_manager();
        let demo_model = self.imp().demo_model.borrow().clone().unwrap();

        let model_set: HashSet<(String, u64), RandomState> = HashSet::from_iter(demo_model.into_iter().map(|d|{
            let demo = d.unwrap().downcast::<DemoObject>().unwrap();
            (demo.name(), demo.size())
        }));
        let data_set: HashSet<(String, u64), RandomState> = HashSet::from_iter(demo_manager.borrow_mut().get_demos().into_iter().map(|t|{
            let demo = t.1;
            (demo.filename.to_owned(), demo.metadata.as_ref().unwrap().len())
        }));
        
        demo_model.retain(|d|{
            let d = d.downcast_ref::<DemoObject>().unwrap();
            data_set.contains(&(d.name(), d.size()))
        });

        let added = data_set.difference(&model_set);
        for dn in added {
            demo_model.append(&DemoObject::new(demo_manager.borrow().get_demo(&dn.0).unwrap()));
        }
    }

    fn update_detail_view(&self){
        self.event_model().remove_all();
        if let Some(demo) = self.get_selected_demo(){
            demo.events.iter().map(EventObject::from).for_each(|e|{
                self.event_model().append(&e)
            });
            self.imp().name_entry.buffer().set_text(&demo.filename);
            self.imp().name_entry.set_icon_activatable(gtk::EntryIconPosition::Secondary, true);
            self.imp().notes_area.buffer().set_text(demo.notes.to_owned().unwrap_or_default().as_str());
            self.imp().detail_box.set_sensitive(true);
            self.imp().event_add.set_sensitive(true);
            self.update_event_selection();
            if let Some(header) = &demo.header {
                self.imp().map_entry.buffer().set_text(&header.map);
                self.imp().nick_entry.buffer().set_text(&header.nick);
                self.imp().duration_entry.buffer().set_text(format!("{} ({} ticks | {:.3} tps)", crate::util::sec_to_timestamp(header.duration), header.ticks, header.ticks as f32/header.duration).as_str());
                self.imp().server_entry.buffer().set_text(&header.server);

                self.imp().duration_label.set_label(&format!("{}\n{}",crate::util::sec_to_timestamp(header.duration), header.ticks));
            }else{
                self.imp().map_entry.buffer().set_text("");
                self.imp().nick_entry.buffer().set_text("");
                self.imp().duration_entry.buffer().set_text("");
                self.imp().server_entry.buffer().set_text("");
                
                self.imp().duration_label.set_label("00:00\n0");
            }
        }else{
            self.imp().name_entry.buffer().set_text("");
            self.imp().map_entry.buffer().set_text("");
            self.imp().nick_entry.buffer().set_text("");
            self.imp().duration_entry.buffer().set_text("");
            self.imp().server_entry.buffer().set_text("");
            self.imp().name_entry.set_icon_activatable(gtk::EntryIconPosition::Secondary, false);
            self.imp().notes_area.buffer().set_text("");
            self.imp().detail_box.set_sensitive(false);

            self.imp().event_remove.set_sensitive(false);
            self.imp().event_edit.set_sensitive(false);
            self.imp().event_add.set_sensitive(false);

            self.imp().duration_label.set_label("00:00\n0");
        }
    }

    fn setup_titlebar_callbacks(&self){
        self.imp().button_open_folder.connect_clicked(clone!(@weak self as wnd => move|_|{
            glib::spawn_future_local(clone!(@weak wnd => async move {
                let dia = FileDialog::builder().build();
                let res = dia.select_folder_future(Some(&wnd)).await;
                if let Ok(file) = res{
                    wnd.settings().borrow_mut().demo_folder_path = file.path().unwrap().display().to_string();
                    wnd.settings().borrow().save();
                    wnd.demo_manager().borrow_mut().load_demos(&wnd.settings().borrow().demo_folder_path).await;
                    wnd.refresh();
                }
            }));
        }));

        self.imp().delete_button.connect_clicked(clone!(@weak self as wnd => move|_|{
            glib::spawn_future_local(clone!(@weak wnd => async move {
    
                {
                    let demos = wnd.demo_manager();
                    let sel_demos = wnd.get_all_selected_demos();
                    if !wnd.delete_dialog().await{
                        return;
                    }
    
                    for d in sel_demos {
                        demos.borrow_mut().delete_demo(&d).await;
                    }
                    
                }
                wnd.update_demos();
            }));
        }));

        self.imp().reload_button.connect_clicked(clone!(@weak self as wnd => move |b|{
            glib::spawn_future_local(clone!(@weak wnd, @weak b => async move {
                b.set_sensitive(false);
                wnd.demo_manager().borrow_mut().load_demos(&wnd.settings().borrow().demo_folder_path).await;
                wnd.refresh();
                b.set_sensitive(true);
            }));
        }));
    }

    fn setup_main_controls(&self){
        self.imp().playbar.set_range(0.0, 100.0);

        self.imp().playbar.connect_value_changed(clone!(@weak self as wnd => move |ph|{
            let tps = wnd.get_current_tps();
            let secs = ticks_to_sec(ph.value() as u32, tps);
            wnd.imp().timestamp_label.set_label(format!("{}\n{}", sec_to_timestamp(secs).as_str(), ph.value() as u32).as_str());
        }));

        self.imp().play_button.connect_clicked(clone!(@weak self as wnd => move |b| {
            glib::spawn_future_local(clone!(@weak wnd, @weak b => async move {
                b.set_sensitive(false);
                let selected = wnd.get_selected_demo().unwrap();
                let _ = wnd.rcon_manager().borrow_mut().play_demo(&selected).await;
                b.set_sensitive(true);
            }));
        }));

        self.imp().seek_button.connect_clicked(clone!(@weak self as wnd => move |b|{
            glib::spawn_future_local(clone!(@weak wnd, @weak b => async move {
                b.set_sensitive(false);
                let _ = wnd.rcon_manager().borrow_mut().skip_to_tick(wnd.imp().playbar.value() as u32, false).await;
                b.set_sensitive(true);
            }));
        }));

        self.imp().stop_playback_button.connect_clicked(clone!(@weak self as wnd => move |b|{
            glib::spawn_future_local(clone!(@weak wnd, @weak b => async move {
                b.set_sensitive(false);
                let _ = wnd.rcon_manager().borrow_mut().stop_playback().await;
                b.set_sensitive(true);
            }));
        }));

        self.imp().skip_backward_button.connect_clicked(clone!(@weak self as wnd => move |_|{
            let tps = wnd.get_current_tps();
            wnd.imp().playbar.set_value(wnd.imp().playbar.value() - 30.0*tps as f64);
        }));

        self.imp().skip_forward_button.connect_clicked(clone!(@weak self as wnd => move |_|{
            let tps = wnd.get_current_tps();
            wnd.imp().playbar.set_value(wnd.imp().playbar.value() + 30.0*tps as f64);
        }));
        
        self.imp().convert_to_replay_button.connect_clicked(clone!(@weak self as wnd => move |_|{
            glib::spawn_future_local(clone!(@weak wnd => async move {
                let mut demo = wnd.get_selected_demo().unwrap();
                let tf_folder_path = async_std::path::PathBuf::from(&wnd.settings().borrow().tf_folder_path);
                if !tf_folder_path.is_dir().await {
                    wnd.notice_dialog("TF2 folder does not exist or cannot be accessed", "Please check your TF2 folder setting").await;
                    return;
                }
                let replay_folder = tf_folder_path.join("tf").join("replay").join("client").join("replays");
                if demo.has_replay(&replay_folder).await {
                    wnd.notice_dialog("Demo already converted", "").await;
                    return;
                }
                match wnd.demo_manager().borrow().convert_to_replay(&replay_folder, &mut demo).await {
                    Ok(_) => wnd.notice_dialog("Replay created successfully", "").await,
                    Err(e) => wnd.notice_dialog("Failed to create replay", &e.to_string()).await,
                };
            }));
        }));

        self.imp().detail_edit_cancel.connect_clicked(clone!(@weak self as wnd => move |_|{
            let val = wnd.imp().playbar.value();
            wnd.refresh();
            wnd.imp().playbar.set_value(val);
        }));

        self.imp().detail_edit_save.connect_clicked(clone!(@weak self as wnd => move |b|{
            glib::spawn_future_local(clone!(@weak wnd, @weak b => async move {
                b.set_sensitive(false);
                let buf = wnd.imp().notes_area.buffer();
                let mut demo = wnd.get_selected_demo().unwrap();
                demo.notes = Some(buf.text(&buf.start_iter(), &buf.end_iter(), true).to_string());
                demo.events.clear();
                wnd.imp().event_model.borrow().clone().unwrap().into_iter().map(|e|{
                    let ev: Event = e.unwrap().downcast_ref::<EventObject>().unwrap().into();
                    ev
                }).for_each(|e|demo.events.push(e));
                demo.save_json().await;
                wnd.demo_manager().borrow_mut().get_demos_mut().insert(demo.filename.clone(), demo);
                b.set_sensitive(true);
                let val = wnd.imp().playbar.value();
                wnd.refresh();
                wnd.imp().playbar.set_value(val);
            }));
        }));

        self.imp().event_remove.connect_clicked(clone!(@weak self as wnd => move|_|{
            let idx = wnd.imp().event_selection.borrow().clone().unwrap().selected();
            wnd.imp().event_model.borrow().clone().unwrap().remove(idx);
            wnd.update_event_selection();
            wnd.imp().detail_edit.set_sensitive(true);
        }));

        self.imp().event_add.connect_clicked(clone!(@weak self as wnd => move|_|{
            let length = wnd.get_selected_demo().unwrap().header.map_or(u32::MAX, |h|h.ticks);
            EventDialog::new(&wnd, "Add event", &EventObject::new("", "Bookmark", wnd.imp().playbar.value() as u32), length)
                .callback(clone!(@weak wnd => move |name, bookmark_type, tick|{
                    wnd.event_model().append(&EventObject::new(name, bookmark_type, tick));
                })).show();
            wnd.imp().detail_edit.set_sensitive(true);
        }));

        self.imp().event_edit.connect_clicked(clone!(@weak self as wnd => move|_|{
            let length = wnd.get_selected_demo().unwrap().header.map_or(u32::MAX, |h|h.ticks);
            let ev = wnd.imp().event_selection.borrow().clone().unwrap().selected_item();
            EventDialog::new(&wnd, "Edit event", ev.and_downcast_ref::<EventObject>().unwrap(), length)
            .callback(clone!(@weak wnd => move |name, bookmark_type, tick|{
                let evobj = wnd.imp().event_selection.borrow().clone().unwrap().selected_item();
                let ev = evobj.and_downcast_ref::<EventObject>().unwrap();
                ev.set_name(name);
                ev.set_bookmark_type(bookmark_type);
                ev.set_tick(tick);
            })).show();
            wnd.imp().detail_edit.set_sensitive(true);
        }));
    }

    fn setup_detail_view(&self){
        self.imp().name_entry.connect_icon_press(clone!(@weak self as wnd => move |_, _| {
            let _ = opener::reveal(wnd.get_selected_demo().unwrap().path.as_path()).inspect_err(|e|log::warn!("{}", e));
        }));

        let model = gtk::gio::ListStore::new::<Object>();
        self.imp().event_model.replace(Some(model));

        let factory = SignalListItemFactory::new();
        factory.connect_setup(clone!(@weak self as wnd => move |_,li|{
            let list_item = li.downcast_ref::<ListItem>().unwrap();

            let name_label = Label::builder().halign(gtk::Align::Start).build();
            list_item.property_expression("item").chain_property::<EventObject>("name").bind(&name_label, "label", Widget::NONE);

            let seek_button = Button::builder().icon_name("find-location-symbolic").tooltip_text("Jump to this event").vexpand(false).valign(gtk::Align::Center).build();
            seek_button.connect_clicked(clone!(@weak wnd, @strong list_item => move |_|{
                glib::spawn_future_local(clone!(@weak wnd, @weak list_item => async move {
                    let offset = (wnd.get_current_tps() * wnd.settings().borrow().event_skip_predelay) as u32;
                    let mut tick = list_item.property::<EventObject>("item").tick();
                    if offset > tick {
                        tick = 0;
                    } else {
                        tick -= offset;
                    }
                    let _ = wnd.rcon_manager().borrow_mut().skip_to_tick(tick, false).await;
                }));
            }));

            let start_box = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(10).margin_start(10).margin_end(20).build();
            start_box.append(&seek_button);
            start_box.append(&name_label);

            let type_label = Label::builder().halign(gtk::Align::Center).justify(gtk::Justification::Center).build();
            list_item.property_expression("item").chain_property::<EventObject>("bookmark-type").bind(&type_label, "label", Widget::NONE);
            
            let time_label = Label::builder().halign(gtk::Align::End).justify(gtk::Justification::Right).margin_end(20).margin_start(20).build();
            list_item.property_expression("item").chain_property::<EventObject>("tick").chain_closure_with_callback(move |v|{
                let tick: u32 = v[1].get().unwrap();
                let secs = crate::util::ticks_to_sec(tick, wnd.get_selected_demo().unwrap().tps().unwrap_or(Demo::TICKRATE));
                format!("{} ({})", crate::util::sec_to_timestamp(secs), tick)
            }).bind(&time_label, "label", Widget::NONE);

            let cbox = CenterBox::builder().start_widget(&start_box).center_widget(&type_label).end_widget(&time_label).hexpand(true).height_request(40).build();
            list_item.set_child(Some(&cbox));
        }));

        let sorted = SortListModel::builder().model(&self.event_model()).sorter(&NumericSorter::builder().expression(&PropertyExpression::new(EventObject::static_type(), None::<Expression>, "tick")).build()).build();
        let sel = SingleSelection::builder().model(&sorted).build();
        self.imp().event_list.set_model(Some(&sel));
        self.imp().event_selection.replace(Some(sel));
        self.imp().event_list.set_factory(Some(&factory));

        self.imp().event_selection.borrow().clone().unwrap().connect_selection_changed(clone!(@weak self as wnd => move |s,_,_|{
            if s.selected_item().is_some(){
                wnd.imp().event_edit.set_sensitive(true);
                wnd.imp().event_remove.set_sensitive(true);
            }else{
                wnd.imp().event_edit.set_sensitive(false);
                wnd.imp().event_remove.set_sensitive(false);
            }
        }));

        self.imp().event_list.connect_activate(clone!(@weak self as wnd => move |_,i|{
            let evob = wnd.event_model().item(i).unwrap().downcast::<EventObject>().unwrap();
            wnd.imp().playbar.set_value(evob.tick() as f64);
        }));

        self.imp().notes_area.buffer().connect_changed(clone!(@weak self as wnd => move |_|{
            wnd.imp().detail_edit.set_sensitive(true);
        }));
    }

    fn setup_demo_list(&self){
        let demo_model = gio::ListStore::new::<DemoObject>();
        let sorted_model = SortListModel::builder().model(&demo_model).build();
        let selection = MultiSelection::new(None::<gio::ListStore>);

        selection.set_model(Some(&sorted_model));
        
        self.imp().demo_list.set_model(Some(&selection));
        self.imp().demo_model.replace(Some(demo_model));
        self.imp().demo_selection.replace(Some(selection));
        
        sorted_model.set_sorter(self.imp().demo_list.sorter().as_ref());

        let name_factory = SignalListItemFactory::new();
        name_factory.connect_setup(|_, li|{
            let listitem = li.downcast_ref::<ListItem>().unwrap();
            let label = Label::builder().halign(gtk::Align::Start).build();
            listitem.set_child(Some(&label));
            listitem.property_expression("item").chain_property::<DemoObject>("name").bind(&label, "label", Widget::NONE);
        });
        self.imp().demo_list.append_column(&ColumnViewColumn::builder()
            .title("Name")
            .resizable(true)
            .factory(&name_factory)
            .expand(true)
            .sorter(
                &StringSorter::new(Some(&
                    PropertyExpression::new(DemoObject::static_type(), None::<Expression>,"name")
                ))
            )
            .build());

        let map_factory = SignalListItemFactory::new();
        map_factory.connect_setup(|_, li|{
            let listitem = li.downcast_ref::<ListItem>().unwrap();
            let label = Label::builder().halign(gtk::Align::Start).build();
            listitem.set_child(Some(&label));
            listitem.property_expression("item").chain_property::<DemoObject>("map").bind(&label, "label", Widget::NONE);
        });
        self.imp().demo_list.append_column(&ColumnViewColumn::builder()
            .title("Map")
            .resizable(true)
            .factory(&map_factory)
            .expand(true)
            .sorter(
                &StringSorter::new(Some(&
                    PropertyExpression::new(DemoObject::static_type(), None::<Expression>,"map")
                ))
            )
            .build());
        
        let username_factory = SignalListItemFactory::new();
        username_factory.connect_setup(|_, li|{
            let listitem = li.downcast_ref::<ListItem>().unwrap();
            let label = Label::builder().halign(gtk::Align::Start).build();
            listitem.set_child(Some(&label));
            listitem.property_expression("item").chain_property::<DemoObject>("username").bind(&label, "label", Widget::NONE);
        });
        self.imp().demo_list.append_column(&ColumnViewColumn::builder()
            .title("Username")
            .resizable(true)
            .factory(&username_factory)
            .expand(true)
            .sorter(
                &StringSorter::new(Some(&
                    PropertyExpression::new(DemoObject::static_type(), None::<Expression>,"username")
                ))
            )
            .build());
        
        let duration_factory = SignalListItemFactory::new();
        duration_factory.connect_setup(|_, li|{
            let listitem = li.downcast_ref::<ListItem>().unwrap();
            let label = Label::builder().halign(gtk::Align::End).build();
            listitem.set_child(Some(&label));
            listitem.property_expression("item").chain_property::<DemoObject>("duration").chain_closure_with_callback(|v|{
                humantime::format_duration(Duration::from_secs(v[1].get::<f32>().unwrap() as u64)).to_string()
            }).bind(&label, "label", Widget::NONE);
        });
        self.imp().demo_list.append_column(&ColumnViewColumn::builder()
            .title("Duration")
            .resizable(true)
            .factory(&duration_factory)
            .expand(true)
            .sorter(
                &NumericSorter::new(Some(&
                    PropertyExpression::new(DemoObject::static_type(), None::<Expression>,"duration")
                ))
            )
            .build());
        
        let date_factory = SignalListItemFactory::new();
        date_factory.connect_setup(|_, li|{
            let listitem = li.downcast_ref::<ListItem>().unwrap();
            let label = Label::builder().halign(gtk::Align::End).build();
            listitem.set_child(Some(&label));
            listitem.property_expression("item").chain_property::<DemoObject>("created").chain_closure_with_callback(|v|{
                chrono::Local.timestamp_millis_opt(v[1].get().unwrap()).unwrap().format("%Y-%m-%d %H:%M:%S").to_string()
            }).bind(&label, "label", Widget::NONE);
        });
        let date_column = &ColumnViewColumn::builder()
            .title("Created")
            .resizable(true)
            .factory(&date_factory)
            .expand(true)
            .sorter(
                &NumericSorter::new(Some(&
                    PropertyExpression::new(DemoObject::static_type(), None::<Expression>,"created")
                ))
            )
            .build();
        
        self.imp().demo_list.append_column(date_column);

        let size_factory = SignalListItemFactory::new();
        size_factory.connect_setup(|_, li|{
            let listitem = li.downcast_ref::<ListItem>().unwrap();
            let label = Label::builder().halign(gtk::Align::End).build();
            listitem.set_child(Some(&label));
            listitem.property_expression("item").chain_property::<DemoObject>("size").chain_closure_with_callback(|v|{
                format!("{:.2}B", size_format::SizeFormatterBinary::new(v[1].get::<u64>().unwrap()))
            }).bind(&label, "label", Widget::NONE);
        });
        self.imp().demo_list.append_column(&ColumnViewColumn::builder()
            .title("Size")
            .resizable(true)
            .factory(&size_factory)
            .expand(true)
            .sorter(
                &NumericSorter::new(Some(&
                    PropertyExpression::new(DemoObject::static_type(), None::<Expression>,"size")
                ))
            )
            .build());

        let bookmark_factory = SignalListItemFactory::new();
        bookmark_factory.connect_setup(|_, li|{
            let listitem = li.downcast_ref::<ListItem>().unwrap();
            let label = Label::builder().halign(gtk::Align::End).build();
            listitem.set_child(Some(&label));
            listitem.property_expression("item").chain_property::<DemoObject>("bookmarks").bind(&label, "label", Widget::NONE);
        });
        self.imp().demo_list.append_column(&ColumnViewColumn::builder()
            .title("Bookmarks")
            .resizable(true)
            .factory(&bookmark_factory)
            .expand(true)
            .sorter(
                &NumericSorter::new(Some(&
                    PropertyExpression::new(DemoObject::static_type(), None::<Expression>,"bookmarks")
                ))
            )
            .build());

        self.imp().demo_list.sort_by_column(Some(date_column), gtk::SortType::Descending);

        self.imp().demo_list.connect_activate(clone!(@weak self as wnd => move |_,_|{
            glib::spawn_future_local(clone!(@weak wnd => async move {
                let demo = wnd.get_selected_demo().unwrap();
                let _ = wnd.rcon_manager().borrow_mut().play_demo(&demo).await;
            }));
        }));

        self.selection().connect_selection_changed(clone!(@weak self as wnd => move|_,_,_|{
            let demo = wnd.get_selected_demo();
            if demo.is_none(){
                wnd.imp().playbar.set_value(0.0);
                wnd.imp().playbar.set_range(0.0, 1.0);
                wnd.imp().playbar.clear_marks();
                wnd.imp().playbar.set_sensitive(false);
                wnd.update_detail_view();
    
                wnd.imp().left_button_box.set_sensitive(false);
                wnd.imp().detail_edit.set_sensitive(false);
                return;
            }
            let demo = demo.unwrap();
            wnd.imp().left_button_box.set_sensitive(true);
            wnd.imp().playbar.set_sensitive(true);
            wnd.update_detail_view();
            wnd.imp().playbar.set_value(0.0);
            wnd.imp().playbar.clear_marks();
            wnd.imp().playbar.set_range(0.0, demo.header.as_ref().map_or(0, |h|h.ticks) as f64);
            for event in &demo.events {
                wnd.imp().playbar.add_mark(event.tick as f64, gtk::PositionType::Bottom, None);
            }
            wnd.imp().detail_edit.set_sensitive(false);
        }));
    }

    pub fn open_settings_on_first_launch(&self){
        if self.settings().borrow().first_launch {
            SettingsWindow::new(&self).show();
        }
    }
}