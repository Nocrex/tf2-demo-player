use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gio::ListModel;
use gtk::glib::Object;
use gtk::{glib, glib::clone};
use gtk::{prelude::*, CenterBox, Entry, Frame, Grid, Label, ListItem, ListView, MultiSelection, Notebook, ScrolledWindow, SignalListItemFactory, TextView, Widget};

use crate::demo_manager::Demo;

use super::event_object::EventObject;

pub fn build_detail_view<F: Fn(u32) + 'static>(event_activated: F) -> (Notebook, Rc<dyn Fn(Option<Demo>)>) {
    let detail_view = Notebook::builder().show_border(true).build();

    let (detail_tab, update_detail_tab) = build_detail_tab();
    detail_view.append_page(&detail_tab, Some(&Label::new(Some(&"Details"))));

    let (event_tab, update_event_tab) = build_event_tab(event_activated);
    detail_view.append_page(&event_tab, Some(&Label::new(Some(&"Events"))));

    (detail_view, Rc::new(move |demo|{
        update_detail_tab(demo.as_ref());
        update_event_tab(demo.as_ref());
    }))
}

fn build_detail_tab() -> (ScrolledWindow, std::boxed::Box<dyn Fn(Option<&Demo>)>) {
    let grid = Grid::builder().column_homogeneous(false).row_homogeneous(false).row_spacing(10).column_spacing(20).margin_end(10).margin_start(10).margin_top(10).margin_bottom(10).build();
    
    grid.attach(&Label::builder().label("Name:").halign(gtk::Align::Start).build(), 0, 0, 1, 1);
    let name_entry = Entry::builder().halign(gtk::Align::Fill).valign(gtk::Align::Center).hexpand(true).editable(false).secondary_icon_sensitive(true).secondary_icon_activatable(true).secondary_icon_name("folder-open").secondary_icon_tooltip_text("Reveal in files").build();
    grid.attach(&name_entry, 1, 0, 1, 1);
    let path = Rc::new(RefCell::new(std::path::PathBuf::new()));
    name_entry.connect_icon_press(clone!(@weak path => move |_, _| {
        let _ = opener::reveal(path.borrow().as_path()).inspect_err(|e|log::warn!("{}", e));
    }));

    grid.attach(&Label::builder().label("Map:").halign(gtk::Align::Start).build(), 0, 1, 1, 1);
    let map_entry = Entry::builder().halign(gtk::Align::Fill).valign(gtk::Align::Center).hexpand(true).editable(false).build();
    grid.attach(&map_entry, 1, 1, 1, 1);

    grid.attach(&Label::builder().label("Username:").halign(gtk::Align::Start).build(), 0, 2, 1, 1);
    let nick_entry = Entry::builder().halign(gtk::Align::Fill).valign(gtk::Align::Center).hexpand(true).editable(false).build();
    grid.attach(&nick_entry, 1, 2, 1, 1);

    grid.attach(&Label::builder().label("Duration:").halign(gtk::Align::Start).build(), 0, 3, 1, 1);
    let dur_entry = Entry::builder().halign(gtk::Align::Fill).valign(gtk::Align::Center).hexpand(true).editable(false).build();
    grid.attach(&dur_entry, 1, 3, 1, 1);
    
    grid.attach(&Label::builder().label("Server:").halign(gtk::Align::Start).build(), 0, 4, 1, 1);
    let server_entry = Entry::builder().halign(gtk::Align::Fill).valign(gtk::Align::Center).hexpand(true).editable(false).build();
    grid.attach(&server_entry, 1, 4, 1, 1);

    grid.attach(&Label::builder().label("Notes:").halign(gtk::Align::Start).build(), 0, 5, 1, 1);
    let notes_area = TextView::builder().margin_end(10).margin_start(10).margin_top(10).margin_bottom(10).build();
    grid.attach(&Frame::builder().child(&notes_area).vexpand(true).valign(gtk::Align::Fill).build(), 0, 6, 2, 1);
    
    let scroller = ScrolledWindow::builder().child(&grid).hexpand(true).vexpand(true).build();
    (scroller, std::boxed::Box::new(clone!(@strong path => move |demo_o|{
        if let Some(demo) = demo_o{
            name_entry.buffer().set_text(&demo.filename);
            name_entry.set_icon_activatable(gtk::EntryIconPosition::Secondary, true);
            path.replace(demo.path.clone());
            notes_area.buffer().set_text(demo.notes.to_owned().unwrap_or_default().as_str());
            if let Some(header) = &demo.header {
                map_entry.buffer().set_text(&header.map);
                nick_entry.buffer().set_text(&header.nick);
                dur_entry.buffer().set_text(format!("{} ({} ticks | {:.3} tps)", crate::util::sec_to_timestamp(header.duration), header.ticks, header.ticks as f32/header.duration).as_str());
                server_entry.buffer().set_text(&header.server);
            }else{
                map_entry.buffer().set_text("");
                nick_entry.buffer().set_text("");
                dur_entry.buffer().set_text("");
                server_entry.buffer().set_text("");
            }
        }else{
            name_entry.buffer().set_text("");
            map_entry.buffer().set_text("");
            nick_entry.buffer().set_text("");
            dur_entry.buffer().set_text("");
            server_entry.buffer().set_text("");
            name_entry.set_icon_activatable(gtk::EntryIconPosition::Secondary, false);
            notes_area.buffer().set_text("");
        }
    })))
}

fn build_event_tab<F: Fn(u32) + 'static>(activation_callback: F) -> (ScrolledWindow, std::boxed::Box<dyn Fn(Option<&Demo>) -> ()>){
    let model = gtk::gio::ListStore::new::<Object>();

    let tps = Rc::new(Cell::new(66.667));

    let factory = SignalListItemFactory::new();
    factory.connect_setup(clone!(@weak tps => move |_,li|{
        let list_item = li.downcast_ref::<ListItem>().unwrap();

        let name_label = Label::builder().halign(gtk::Align::Start).margin_start(20).build();
        list_item.property_expression("item").chain_property::<EventObject>("name").bind(&name_label, "label", Widget::NONE);

        let type_label = Label::builder().halign(gtk::Align::Center).justify(gtk::Justification::Center).build();
        list_item.property_expression("item").chain_property::<EventObject>("bookmark-type").bind(&type_label, "label", Widget::NONE);
        
        let time_label = Label::builder().halign(gtk::Align::End).justify(gtk::Justification::Right).margin_end(20).build();
        list_item.property_expression("item").chain_property::<EventObject>("tick").chain_closure_with_callback(move |v|{
            let tick: u32 = v[1].get().unwrap();
            let secs = crate::util::ticks_to_sec(tick, tps.get());
            crate::util::sec_to_timestamp(secs)
        }).bind(&time_label, "label", Widget::NONE);

        let cbox = CenterBox::builder().start_widget(&name_label).center_widget(&type_label).end_widget(&time_label).hexpand(true).height_request(40).build();
        list_item.set_child(Some(&cbox));
    }));

    let sel = MultiSelection::new(None::<ListModel>);
    sel.set_model(Some(&model));
    
    let list = ListView::builder().model(&sel).factory(&factory).build();

    list.connect_activate(clone!(@weak model => move |_,i|{
        let evob = model.item(i).unwrap().downcast::<EventObject>().unwrap();
        activation_callback(evob.tick());
    }));

    let scroller = ScrolledWindow::builder().child(&list).hexpand(true).vexpand(true).build();

    (scroller, std::boxed::Box::new(clone!(@strong tps, @strong model => move |demo|{
        model.remove_all();
        if let Some(demo) = demo {
            if let Some(header) = &demo.header {
                tps.set(header.ticks as f32/header.duration);
            }
            demo.events.iter().map(EventObject::new).for_each(|e|{
                model.append(&e)
            });
        }
    })))
}