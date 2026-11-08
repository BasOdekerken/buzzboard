mod connection;
mod display;
mod host;

use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::display::Display;
use crate::host::Host;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| "Not found">
                <Route path=path!("/display") view=Display />
                <Route path=path!("/host") view=Host />
                <Route path=path!("/play") view=|| view! { <h1>"Play"</h1> } />
            </Routes>
        </Router>
    }
}
