use istyles::istyles;
use leptos::prelude::*;

istyles!(styles, "assets/module.postcss/icon.module.css.map");

pub fn build_icon() -> AnyView {
    view! {
        <svg
            class=styles::icon
            height="14"
            viewBox="8 4 10 16"
            width="12"
            xmlns="http://www.w3.org/2000/svg"
        >
            <path d="M8 5v14l11-7z" />
        </svg>
    }
    .into_any()
}

pub fn config_icon() -> AnyView {
    view! {
        <svg
            class=styles::icon
            height="15"
            viewBox="0 0 24 24"
            width="15"
            xmlns="http://www.w3.org/2000/svg"
        >
            <path d="M19.43 12.98c.04-.32.07-.64.07-.98s-.03-.66-.07-.98l2.11-1.65c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.3-.61-.22l-2.49 1c-.52-.4-1.08-.73-1.69-.98l-.38-2.65C14.46 2.18 14.25 2 14 2h-4c-.25 0-.46.18-.49.42l-.38 2.65c-.61.25-1.17.59-1.69.98l-2.49-1c-.23-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64l2.11 1.65c-.04.32-.07.65-.07.98s.03.66.07.98l-2.11 1.65c-.19.15-.24.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1c.52.4 1.08.73 1.69.98l.38 2.65c.03.24.24.42.49.42h4c.25 0 .46-.18.49-.42l.38-2.65c.61-.25 1.17-.59 1.69-.98l2.49 1c.23.09.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.65zM12 15.5c-1.93 0-3.5-1.57-3.5-3.5s1.57-3.5 3.5-3.5 3.5 1.57 3.5 3.5-1.57 3.5-3.5 3.5z" />
        </svg>
    }.into_any()
}

pub fn expandable_icon() -> AnyView {
    view! {
        <svg
            class=styles::icon
            height="10"
            viewBox="6 8 12 8"
            width="10"
            opacity="0.5"
            xmlns="http://www.w3.org/2000/svg"
        >
            <path d="M16.59 8.59L12 13.17 7.41 8.59 6 10l6 6 6-6z" />
        </svg>
    }
    .into_any()
}

pub fn checkmark_icon() -> AnyView {
    view! {
        <svg
            class=styles::icon
            height="18"
            viewBox="2 2 22 22"
            width="18"
            xmlns="http://www.w3.org/2000/svg"
        >
            <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" />
        </svg>
    }
    .into_any()
}

pub fn clipboard_icon() -> AnyView {
    view! {
        <svg
            class=styles::icon
            height="18"
            width="18"
            viewBox="0 0 24 24"
            xmlns="http://www.w3.org/2000/svg"
        >
            <rect x="7" y="15" width="7" height="2" />
            <rect x="7" y="11" width="10" height="2" />
            <rect x="7" y="7" width="10" height="2" />
            <path d="M19,3L19,3h-4.18C14.4,1.84,13.3,1,12,1c-1.3,0-2.4,0.84-2.82,2H5h0C4.86,3,4.73,3.01,4.6,3.04 C4.21,3.12,3.86,3.32,3.59,3.59c-0.18,0.18-0.33,0.4-0.43,0.64C3.06,4.46,3,4.72,3,5v14c0,0.27,0.06,0.54,0.16,0.78 c0.1,0.24,0.25,0.45,0.43,0.64c0.27,0.27,0.62,0.47,1.01,0.55C4.73,20.99,4.86,21,5,21h0h14h0c1.1,0,2-0.9,2-2V5 C21,3.9,20.1,3,19,3z M12,2.75c0.41,0,0.75,0.34,0.75,0.75c0,0.41-0.34,0.75-0.75,0.75c-0.41,0-0.75-0.34-0.75-0.75 C11.25,3.09,11.59,2.75,12,2.75z M19,19H5V5h14V19z" />
        </svg>
    }.into_any()
}

pub fn more_options_icon() -> AnyView {
    view! {
        <svg
            class=styles::icon
            height="18"
            viewBox="0 0 24 24"
            width="18"
            xmlns="http://www.w3.org/2000/svg"
        >
            <path d="M6 10c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm12 0c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm-6 0c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z" />
        </svg>
    }.into_any()
}
