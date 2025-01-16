use std::time::Duration;

use chrono::TimeZone;
use gtk::gio::ListStore;
use gtk::{prelude::*, ColumnView, ColumnViewColumn, Expression, Label, ListItem, MultiSelection, NumericSorter, PropertyExpression, ScrolledWindow, SignalListItemFactory, SingleSelection, SortListModel, StringSorter, Widget};

use super::DemoObject;

pub fn build_demo_list() -> (ScrolledWindow, MultiSelection){
    let demo_model = ListStore::new::<DemoObject>();
    let sorted_model = SortListModel::builder().model(&demo_model).build();

    let selection = MultiSelection::new(None::<ListStore>);
    selection.set_model(Some(&sorted_model));

    let demo_list = ColumnView::builder()
        .vexpand(true)
        .hexpand(true)
        .model(&selection).build();

    sorted_model.set_sorter(demo_list.sorter().as_ref());

    let name_factory = SignalListItemFactory::new();
    name_factory.connect_setup(|_, li|{
        let listitem = li.downcast_ref::<ListItem>().unwrap();
        let label = Label::builder().halign(gtk::Align::Start).build();
        listitem.set_child(Some(&label));
        listitem.property_expression("item").chain_property::<DemoObject>("name").bind(&label, "label", Widget::NONE);
    });
    demo_list.append_column(&ColumnViewColumn::builder()
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
    demo_list.append_column(&ColumnViewColumn::builder()
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
    demo_list.append_column(&ColumnViewColumn::builder()
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
    demo_list.append_column(&ColumnViewColumn::builder()
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
    demo_list.append_column(&ColumnViewColumn::builder()
        .title("Created")
        .resizable(true)
        .factory(&date_factory)
        .expand(true)
        .sorter(
            &NumericSorter::new(Some(&
                PropertyExpression::new(DemoObject::static_type(), None::<Expression>,"created")
            ))
        )
        .build());

    let size_factory = SignalListItemFactory::new();
    size_factory.connect_setup(|_, li|{
        let listitem = li.downcast_ref::<ListItem>().unwrap();
        let label = Label::builder().halign(gtk::Align::End).build();
        listitem.set_child(Some(&label));
        listitem.property_expression("item").chain_property::<DemoObject>("size").chain_closure_with_callback(|v|{
            format!("{:.2}B", size_format::SizeFormatterBinary::new(v[1].get::<u64>().unwrap()))
        }).bind(&label, "label", Widget::NONE);
    });
    demo_list.append_column(&ColumnViewColumn::builder()
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
    demo_list.append_column(&ColumnViewColumn::builder()
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


    let demo_scroll = ScrolledWindow::builder().has_frame(true).hscrollbar_policy(gtk::PolicyType::Never).child(&demo_list).height_request(150).build();

    (demo_scroll, selection)
}