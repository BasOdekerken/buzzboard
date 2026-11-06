use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| "Not found">
                <Route path=path!("/display") view=|| view! { <h1>"Display"</h1> } />
                <Route path=path!("/host") view=|| view! { <h1>"Host"</h1> } />
                <Route path=path!("/play") view=|| view! { <h1>"Play"</h1> } />
            </Routes>
        </Router>
    }
}
