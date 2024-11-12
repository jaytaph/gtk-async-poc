mod tab_manager;
mod fetcher;

use std::sync::OnceLock;
use std::time::Duration;
use async_channel::Sender;
use gtk4::{Application, ApplicationWindow, Label, ListBox, Notebook, TextView};
use gtk4::glib::{clone, spawn_future_local};
use gtk4::prelude::{ApplicationExt, ApplicationExtManual, BoxExt, ButtonExt, EntryExt, GtkWindowExt, TextBufferExt, TextViewExt, WidgetExt};
use tokio::runtime::Runtime;
use tokio::time::sleep;
use uuid::Uuid;
use crate::tab_manager::{TabInfo, TabManager};
use gtk4::prelude::EditableExt;

const APP_ID: &str = "io.gosub.browser-gtk";

// Have a separate tokio runtime. We theoretically could use the gtk event loop, but
// our "event-loop-poc" uses tokio as well.. so let's try it with this.
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Setting up tokio runtime needs to succeed.")
    })
}

fn tab_manager() -> &'static TabManager {
    static TAB_MANAGER: OnceLock<TabManager> = OnceLock::new();
    TAB_MANAGER.get_or_init(|| {
        TabManager::new()
    })
}

enum Message {
    /// Sent when a favicon has been loaded for tab X
    FaviconLoaded(Uuid, Vec<u8>),
    // Sent when a URL has been loaded for tab X
    UrlLoaded(Uuid, String),
    // Single message to print in the log
    Message(String),
    // Open a new tab
    OpenTab(Uuid, String),
}

fn main() {
    colog::init();

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}


fn build_ui(app: &Application) {
    // Set up a channel to receive the messages for the listbox
    let (sender, receiver) = async_channel::unbounded::<Message>();
    let sender_clone = sender.clone();

    // Widget Listbox will be used to display messages. They come from async tasks.
    let list = ListBox::builder()
        .height_request(200)
        .build();
    let label = Label::new(Some("Ready for action..."));
    label.set_halign(gtk4::Align::Start);
    list.append(&label);

    // Widget Notebook for tabs and pages
    let notebook = Notebook::builder().vexpand(true).build();

    // Widget address bar
    let entry = gtk4::Entry::builder().build();
    entry.connect_activate(move |entry| {
        let url = entry.text().to_string();

        // When entry is "entered", we spawn a tokio task that will load stuff and send messages to the listbox.
        runtime().spawn(clone!(
            #[strong]
            sender,
            async move {
                sender.send(Message::Message("Button clicked".into())).await.unwrap();

                let tab_id = Uuid::new_v4();
                sender.send(Message::OpenTab(tab_id, url.into())).await.unwrap();
            }
        ));
    });

    // Main container
    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();

    vbox.append(&entry);
    vbox.append(&notebook);
    vbox.append(&list);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("My Tokio Poc")
        .default_height(600)
        .default_width(800)
        .child(&vbox)
        .build();

    window.present();

    // Spawn a task ON THE GTK EVENTLOOP, that will wait for messages and process them
    let list_clone = list.clone();
    let notebook_clone = notebook.clone();
    spawn_future_local(async move {
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    handle_messages(message, &sender_clone, &list_clone, &notebook_clone).await;
                }
                Err(_) => return,
            }
        }
    });
}

async fn handle_messages(message: Message, sender: &Sender<Message>, list_clone: &ListBox, notebook_clone: &Notebook) {
    match message {
        Message::FaviconLoaded(tab_id, favicon) => {
            if favicon.is_empty() {
                let message = format!("[{}] Favicon failed to load", tab_id.to_string());

                let label = Label::new(Some(message.as_str()));
                label.set_halign(gtk4::Align::Start);
                list_clone.insert(&label, -1);
            } else {
                let message = format!("[{}] Favicon loaded ({} bytes)", tab_id.to_string(), favicon.len());
                let label = Label::new(Some(message.as_str()));
                label.set_halign(gtk4::Align::Start);
                list_clone.insert(&label, -1);

                // Update favicon
                if let Some(mut tab_info) = tab_manager().get_tab_info(tab_id) {
                    let page_num = tab_manager().get_by_tab(tab_id).unwrap();
                    tab_info.favicon = favicon.clone();
                    tab_manager().add(tab_id, page_num, tab_info);
                }
            }

            if let Some(tab_info) = tab_manager().get_tab_info(tab_id) {
                let tab_label = create_tab_label(false, &tab_info);

                let page_num = tab_manager().get_by_tab(tab_id).unwrap();
                let page_child = notebook_clone.nth_page(Some(page_num)).unwrap();
                notebook_clone.set_tab_label(&page_child, Some(&tab_label));
            }
        }
        Message::UrlLoaded(tab_id, html) => {
            let message = format!("[{}] URL loaded ({} bytes)", tab_id.to_string(), html.len());
            let label = Label::new(Some(message.as_str()));
            label.set_halign(gtk4::Align::Start);
            list_clone.insert(&label, -1);

            // In case the tab is still spinning, we update the tab label
            if let Some(tab_info) = tab_manager().get_tab_info(tab_id) {
                let tab_label = create_tab_label(false, &tab_info);

                let scrolled_window = gtk4::ScrolledWindow::builder()
                    .hscrollbar_policy(gtk4::PolicyType::Never)
                    .vscrollbar_policy(gtk4::PolicyType::Automatic)
                    .build();

                let content = TextView::builder()
                    .editable(false)
                    .wrap_mode(gtk4::WrapMode::Word)
                    .build();
                content.buffer().set_text(&html);
                scrolled_window.set_child(Some(&content));

                let page_num = tab_manager().get_by_tab(tab_id).unwrap();
                notebook_clone.remove_page(Some(page_num));
                notebook_clone.insert_page(&scrolled_window, Some(&tab_label), Some(page_num));
            }
        }
        Message::Message(msg) => {
            let label = Label::new(Some(msg.as_str()));
            label.set_halign(gtk4::Align::Start);
            list_clone.insert(&label, -1);
        }
        Message::OpenTab(tab_id, url) => {
            if tab_manager().get_by_tab(tab_id).is_some() {
                let message = format!("[{}] Tab already open", tab_id.to_string());
                let label = Label::new(Some(message.as_str()));
                label.set_halign(gtk4::Align::Start);
                list_clone.insert(&label, -1);

                return;
            }

            let tab_info = TabInfo::new(tab_id, url.clone());
            let tab_label = create_tab_label(true, &tab_info);

            // Get new page from notebook
            let page_num = notebook_clone.append_page(
                &Label::new(Some(format!("This page contains {}", url).as_str())),
                Some(&tab_label),
            );
            tab_manager().add(tab_id, page_num, tab_info);

            let message = format!("[{}] Opened new tab to load: {}", tab_id.to_string(), url);
            let label = Label::new(Some(message.as_str()));
            label.set_halign(gtk4::Align::Start);
            list_clone.insert(&label, -1);

            // Open favicon and load the url. Both are async tasks
            load_favicon_async(tab_id, sender.clone(), url.as_str());
            load_url_async(tab_id, sender.clone(), url.as_str());
        }
    }
}

fn create_tab_label(loading: bool, tab_info: &TabInfo) -> gtk4::Box {
    let label_vbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);

    if loading {
        let spinner = gtk4::Spinner::new();
        spinner.start();
        label_vbox.append(&spinner);
    } else if !&tab_info.favicon.is_empty() {
        let pixbuf = fetcher::bytes_to_pixbuf(tab_info.favicon.clone()).unwrap();
        let image = gtk4::Image::from_pixbuf(Some(&pixbuf));
        label_vbox.append(&image);
    }

    let title = tab_info.title.clone().unwrap_or_else(|| "...".to_string());
    let tab_label = gtk4::Label::new(Some(&title));
    label_vbox.append(&tab_label);

    let tab_close_button = gtk4::Button::builder()
        .halign(gtk4::Align::End)
        .has_frame(false)
        .margin_bottom(0)
        .margin_end(0)
        .margin_start(0)
        .margin_top(0)
        .child(&gtk4::Image::from_icon_name("window-close-symbolic"))
        .build();
    label_vbox.append(&tab_close_button);

    label_vbox
}

fn load_favicon_async(tab_id: Uuid, sender: Sender<Message>, url: &str) {
    let url = url.to_string();
    runtime().spawn(clone!(
        #[strong]
        sender,
        async move {
            sender.send(Message::Message("Loading favicon, spinner=true".to_string())).await.unwrap();
            sleep(Duration::from_secs(2)).await;

            let favicon = fetcher::fetch_favicon(url.as_str()).await;
            sender.send(Message::FaviconLoaded(tab_id, favicon)).await.unwrap();
        }
    ));
}

fn load_url_async(tab_id: Uuid, sender: Sender<Message>, url: &str) {
    let url = url.to_string();
    runtime().spawn(clone!(
        #[strong]
        sender,
        async move {
            sender.send(Message::Message(format!("Loading URL: {}", url).to_string())).await.unwrap();
            sleep(Duration::from_secs(4)).await;

            match fetcher::fetch_url_body(url.as_str()).await.ok() {
                Some(content) => {
                    let html_content = String::from_utf8(content).unwrap();
                    sender.send(Message::UrlLoaded(tab_id, html_content)).await.unwrap();
                }
                None => {
                    sender.send(Message::Message("Failed to load URL".to_string())).await.unwrap();
                }
            }
        }
    ));
}