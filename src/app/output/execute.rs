use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;

use crate::{
    SQLiteStatementResult,
    app::{
        output::{header::Header, loader::Loader, section::Section},
        state::{GlobalState, GlobalStateStoreFields},
    },
};

istyles!(
    styles,
    "assets/module.postcss/output/execute.module.css.map"
);

const PAGE_SIZE: usize = 100;

#[component]
fn StatementOutput(index: usize, columns: Vec<String>, row_count: usize) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let page = RwSignal::new(0usize);
    let page_count = row_count.div_ceil(PAGE_SIZE);
    let rows = move || {
        let start = page.get() * PAGE_SIZE;
        state.output().with_untracked(|output| {
            let SQLiteStatementResult::Step(table) = &output[index] else {
                unreachable!()
            };
            table
                .values
                .as_ref()
                .unwrap()
                .rows
                .iter()
                .skip(start)
                .take(PAGE_SIZE)
                .map(|row| {
                    view! {
                        <tr>
                            {row.iter().map(|value| {
                                view! { <td class=styles::tdAndTh>{value.clone()}</td> }
                            }).collect_view()}
                        </tr>
                    }
                })
                .collect_view()
        })
    };

    let label = format!("Statement #{}", index + 1);
    let pagination = (row_count > PAGE_SIZE).then(|| {
        view! {
            <nav class=styles::pagination aria-label=format!("{label} pages")>
                <span aria-live="polite">
                    {move || format!(
                        "Rows {}–{} of {row_count}",
                        page.get() * PAGE_SIZE + 1,
                        ((page.get() + 1) * PAGE_SIZE).min(row_count),
                    )}
                </span>
                <div class=styles::pageButtons>
                    <button
                        type="button"
                        disabled=move || page.get() == 0
                        on:click=move |_| page.update(|page| *page -= 1)
                    >
                        "Previous"
                    </button>
                    <span>{move || format!("Page {} of {page_count}", page.get() + 1)}</span>
                    <button
                        type="button"
                        disabled=move || page.get() + 1 == page_count
                        on:click=move |_| page.update(|page| *page += 1)
                    >
                        "Next"
                    </button>
                </div>
            </nav>
        }
    });

    view! {
        <Section label=label.clone()>
            {pagination}
            <div class=styles::tableScroll>
                <table class=styles::table aria-label=label>
                    <thead>
                        <tr>
                            {columns
                                .into_iter()
                                .map(|column| {
                                    view! { <th scope="col" class=styles::tdAndTh>{column}</th> }
                                })
                                .collect_view()}
                        </tr>
                    </thead>
                    <tbody>{rows}</tbody>
                </table>
            </div>
        </Section>
    }
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
                        SQLiteStatementResult::Step(table) => match &table.values {
                            Some(values) => view! {
                                <StatementOutput
                                    index=index
                                    columns=values.columns.clone()
                                    row_count=values.rows.len()
                                />
                            }.into_any(),
                            None => ().into_any(),
                        },
                    })
                    .collect_view()
            }}
        </>
    }
}
