use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;

use crate::{
    SQLiteStatementResult, SQLiteStatementTable,
    app::{
        output::{header::Header, loader::Loader, section::Section},
        state::{GlobalState, GlobalStateStoreFields},
    },
};

istyles!(
    styles,
    "assets/module.postcss/output/execute.module.css.map"
);

fn statement_output(table: &SQLiteStatementTable, index: usize) -> AnyView {
    let Some(values) = &table.values else {
        return ().into_any();
    };

    let label = format!("Statement #{}", index + 1);
    let output = view! {
        <div class=styles::tableScroll>
            <table class=styles::table aria-label=label.clone()>
                <thead>
                    <tr>
                        {values
                            .columns
                            .iter()
                            .map(|column| {
                                view! { <th scope="col" class=styles::tdAndTh>{column.clone()}</th> }
                            })
                            .collect_view()}
                    </tr>
                </thead>
                <tbody>
                    {values
                        .rows
                        .iter()
                        .map(|row| {
                            view! {
                                <tr>
                                    {row
                                        .iter()
                                        .map(|value| {
                                            view! { <td class=styles::tdAndTh>{value.clone()}</td> }
                                        })
                                        .collect_view()}
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    };

    view! { <Section label=label>{output}</Section> }.into_any()
}

#[component]
pub fn Execute() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    view! {
        <>
            <Show when=move || state.is_running().get()>
                <Loader />
            </Show>
            {move || {
                state
                    .output()
                    .read()
                    .iter()
                    .enumerate()
                    .map(|(index, result)| match result {
                        SQLiteStatementResult::Finish => {
                            view! { <Header label="Finished".into() /> }.into_any()
                        }
                        SQLiteStatementResult::Step(table) => statement_output(table, index),
                    })
                    .collect_view()
            }}
        </>
    }
}
