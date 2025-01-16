use gtk::gio::ListStore;
use gtk::{prelude::* , ColumnView, ColumnViewColumn, Label, ListItem, ScrolledWindow, SignalListItemFactory, SingleSelection, Widget};

use super::DemoObject;

pub fn build_demo_list() -> (ScrolledWindow, SingleSelection){
    let demo_model = ListStore::new::<DemoObject>();

    let selection = SingleSelection::builder().model(&demo_model).build();

    let demo_list = ColumnView::builder()
        .vexpand(true)
        .hexpand(true)
        .model(&selection).build();

    let name_factory = SignalListItemFactory::new();
    name_factory.connect_setup(|_, li|{
        let listitem = li.downcast_ref::<ListItem>().unwrap();
        let label = Label::builder().halign(gtk::Align::Start).build();
        listitem.set_child(Some(&label));
        listitem.property_expression("item").chain_property::<DemoObject>("name").bind(&label, "label", Widget::NONE);
    });
    
    demo_list.append_column(&ColumnViewColumn::builder().title("Name").resizable(true).factory(&name_factory).expand(true).build());

    let map_factory = SignalListItemFactory::new();
    map_factory.connect_setup(|_, li|{
        let listitem = li.downcast_ref::<ListItem>().unwrap();
        let label = Label::builder().halign(gtk::Align::Start).build();
        listitem.set_child(Some(&label));
        listitem.property_expression("item").chain_property::<DemoObject>("map").bind(&label, "label", Widget::NONE);
    });
    demo_list.append_column(&ColumnViewColumn::builder().title("Map").resizable(true).factory(&map_factory).expand(true).build());
    
    let username_factory = SignalListItemFactory::new();
    username_factory.connect_setup(|_, li|{
        let listitem = li.downcast_ref::<ListItem>().unwrap();
        let label = Label::builder().halign(gtk::Align::Start).build();
        listitem.set_child(Some(&label));
        listitem.property_expression("item").chain_property::<DemoObject>("username").bind(&label, "label", Widget::NONE);
    });

    demo_list.append_column(&ColumnViewColumn::builder().title("Username").resizable(true).factory(&username_factory).expand(true).build());
    
    let duration_factory = SignalListItemFactory::new();
    duration_factory.connect_setup(|_, li|{
        let listitem = li.downcast_ref::<ListItem>().unwrap();
        let label = Label::builder().halign(gtk::Align::End).build();
        listitem.set_child(Some(&label));
        listitem.property_expression("item").chain_property::<DemoObject>("duration").bind(&label, "label", Widget::NONE);
    });
    
    demo_list.append_column(&ColumnViewColumn::builder().title("Duration").resizable(true).factory(&duration_factory).expand(true).build());
    
    let date_factory = SignalListItemFactory::new();
    date_factory.connect_setup(|_, li|{
        let listitem = li.downcast_ref::<ListItem>().unwrap();
        let label = Label::builder().halign(gtk::Align::End).build();
        listitem.set_child(Some(&label));
        listitem.property_expression("item").chain_property::<DemoObject>("created").bind(&label, "label", Widget::NONE);
    });
    demo_list.append_column(&ColumnViewColumn::builder().title("Created").resizable(true).factory(&date_factory).expand(true).build());

    let size_factory = SignalListItemFactory::new();
    size_factory.connect_setup(|_, li|{
        let listitem = li.downcast_ref::<ListItem>().unwrap();
        let label = Label::builder().halign(gtk::Align::End).build();
        listitem.set_child(Some(&label));
        listitem.property_expression("item").chain_property::<DemoObject>("size").bind(&label, "label", Widget::NONE);
    });
    demo_list.append_column(&ColumnViewColumn::builder().title("Size").resizable(true).factory(&size_factory).expand(true).build());

    let bookmark_factory = SignalListItemFactory::new();
    bookmark_factory.connect_setup(|_, li|{
        let listitem = li.downcast_ref::<ListItem>().unwrap();
        let label = Label::builder().halign(gtk::Align::End).build();
        listitem.set_child(Some(&label));
        listitem.property_expression("item").chain_property::<DemoObject>("bookmarks").bind(&label, "label", Widget::NONE);
    });
    demo_list.append_column(&ColumnViewColumn::builder().title("Bookmarks").resizable(true).factory(&bookmark_factory).expand(true).build());


    let demo_scroll = ScrolledWindow::builder().has_frame(true).hscrollbar_policy(gtk::PolicyType::Never).child(&demo_list).height_request(150).build();

    (demo_scroll, selection)
}