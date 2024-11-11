use std::sync::OnceLock;
use std::time::Duration;
use async_channel::Sender;
use gtk4::{Application, ApplicationWindow, Button, Label, ListBox};
use gtk4::glib::{clone, spawn_future_local};
use gtk4::prelude::{ApplicationExt, ApplicationExtManual, BoxExt, ButtonExt, GtkWindowExt};
use tokio::runtime::Runtime;
use tokio::time::sleep;

const APP_ID: &str = "io.gosub.browser-gtk";

// Have a separate tokio runtime. We theoretically could use the gtk event loop, but
// our "event-loop-poc" uses tokio as well.. so let's try it with this.
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Setting up tokio runtime needs to succeed.")
    })
}

fn main() {
    colog::init();

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    // Listbox will be used to display messages. They come from async tasks.
    let list = ListBox::builder().build();
    let label = Label::new(Some("Ready for action..."));
    list.append(&label);

    // Setup a channel to receive the messages for the listbox
    let (sender, receiver) = async_channel::unbounded::<String>();

    // Spawn a task ON THE GTK EVENTLOOP, that will wait for messages and insert them into the listbox.
    let list_clone = list.clone();
    spawn_future_local(async move {
        while let Ok(message) = receiver.recv().await {
            let label = Label::new(Some(message.as_str()));
            list_clone.insert(&label, -1);
        }
    });

    let button = Button::builder().label("Click me!").build();
    button.connect_clicked(move |_| {
        // When button is clicked, we spawn a tokio task that will load stuff and send messages to the listbox.
        runtime().spawn(clone!(
            #[strong]
            sender,
            async move {
                sender.send("Button clicked".into()).await.unwrap();
                load_favicon(&sender);
                load_url(&sender);
            }
        ));
    });

    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();

    vbox.append(&button);
    vbox.append(&list);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("My Tokio Poc")
        .default_height(600)
        .default_width(800)
        .child(&vbox)
        .build();

    window.present();
}

fn load_favicon(sender: &Sender<String>) {
    runtime().spawn(clone!(
        #[strong]
        sender,
        async move {
            sender.send("Loading favicon, spinner=true".to_string()).await.unwrap();
            sleep(Duration::from_secs(2)).await;

            let favicon = reqwest::get("https://www.google.com/favicon.ico")
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap();

            let s = format!("Favicon loaded ({} bytes) spinner=false", favicon.len());
            sender.send(s).await.unwrap();
        }
    ));
}

fn load_url(sender: &Sender<String>) {
    runtime().spawn(clone!(
        #[strong]
        sender,
        async move {
            sender.send("Loading URL".to_string()).await.unwrap();
            sleep(Duration::from_secs(4)).await;

            let body = reqwest::get("https://www.google.com")
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            let s = format!("URL loaded ({} bytes)", body.len());
            sender.send(s).await.unwrap();
        }
    ));
}