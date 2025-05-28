use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;

use crate::app::{
    output::{header::Header, loader::Loader, section::Section, simple_pane::SimplePane},
    state::{GlobalState, GlobalStateStoreFields},
};
use crate::{SQLiteStatementResult, SQLiteStatementTable};

istyles!(
    styles,
    "assets/module.postcss/output/execute.module.css.map"
);

fn get_output(table: &SQLiteStatementTable) -> Option<AnyView> {
    let Some(values) = &table.values else {
        return None;
    };
    Some(
        view! {
            <table
                class=styles::table
                style="table-layout: fixed; width: 100%; word-wrap: break-word;"
            >
                <tr>
                    {values
                        .columns
                        .iter()
                        .map(|s| {
                            view! { <th class=styles::tdAndTh>{s.to_string()}</th> }
                        })
                        .collect_view()}
                </tr>
                {values
                    .rows
                    .iter()
                    .map(|row| {
                        view! {
                            <tr>

                                {row
                                    .iter()
                                    .map(|s| {
                                        view! { <td class=styles::tdAndTh>{s.to_string()}</td> }
                                    })
                                    .collect_view()}
                            </tr>
                        }
                    })
                    .collect_view()}
            </table>
        }
        .into_any(),
    )
}

#[component]
fn Output() -> AnyView {
    let state = expect_context::<Store<GlobalState>>();

    view! {
        <>
            <Show
                when=move || {
                    state
                        .output()
                        .read()
                        .last()
                        .is_none_or(|r| !matches!(r, SQLiteStatementResult::Finish))
                }
                fallback=|| ()
            >
                <Loader />
            </Show>

            <>
                {move || {
                    state
                        .output()
                        .read()
                        .iter()
                        .enumerate()
                        .map(|(idx, item)| {
                            match &item {
                                SQLiteStatementResult::Finish => {

                                    view! { <Header label="Finished".into() /> }
                                        .into_any()
                                }
                                SQLiteStatementResult::Step(table) => {
                                    let label = format!("Statement #{}", idx + 1);
                                    if let Some(output) = get_output(table) {
                                        view! {
                                            <Section label=label>
                                                <p>{output}</p>
                                            </Section>
                                        }
                                            .into_any()
                                    } else {
                                        ().into_any()
                                    }
                                }
                            }
                        })
                        .collect_view()
                }}
            </>
        </>
    }
    .into_any()
}

#[component]
pub fn Execute() -> impl IntoView {
    view! {
        <SimplePane>
            <Output />
        </SimplePane>
    }
}
