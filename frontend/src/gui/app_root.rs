use std::collections::VecDeque;

use std::rc::Rc;

use gloo_net::http::Request;
use serde_json::value::Value;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use yew_bootstrap::util::*;

use common::dto::result::ResultDTO;

use crate::gui::renderers::*;
use crate::worker::{BenchmarkInput, BenchmarkResult, BenchmarkType, BenchmarkWorker};

pub enum AppRootMessage {
    ChangeModel(String),
    StartBenchmarks,
    BenchmarkComplete(BenchmarkResult),
    BenchmarksFinished(u16, String),
}

pub enum ExperimentResult {
    NotStarted,
    Running,
    Success,
    Error,
}

pub struct AppRoot {
    bridge: Box<dyn Bridge<BenchmarkWorker>>,

    model_input: String,
    status_label: String,
    button_disabled: bool,
    input_disabled: bool,
    total_benchmarks: usize,
    finished_benchmarks: usize,

    experiment_result: ExperimentResult,

    benchmark_results: Vec<BenchmarkResult>,
    remaining_benchmarks: VecDeque<BenchmarkType>,
}

impl Component for AppRoot {
    type Message = AppRootMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let worker_result_callback =
            move |result| link.send_message(AppRootMessage::BenchmarkComplete(result));

        AppRoot {
            bridge: BenchmarkWorker::bridge(Rc::new(worker_result_callback)),
            model_input: String::default(),
            status_label: String::default(),
            button_disabled: false,
            input_disabled: false,
            benchmark_results: Vec::new(),
            remaining_benchmarks: VecDeque::new(),
            total_benchmarks: 0,
            finished_benchmarks: 0,
            experiment_result: ExperimentResult::NotStarted,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppRootMessage::ChangeModel(new_model) => {
                self.model_input = new_model;
                true
            }
            AppRootMessage::StartBenchmarks => {
                self.start_benchmarks();
                true
            }
            AppRootMessage::BenchmarkComplete(result) => {
                self.handle_benchmark_complete(ctx, result);
                true
            }
            AppRootMessage::BenchmarksFinished(status, status_text) => {
                self.handle_benchmarks_finished(status, status_text);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let button_disabled = self.button_disabled || self.model_input.is_empty();

        html! {
        <>
            {include_cdn()}
            {render_main_container(
                &self.model_input,
                self.input_disabled,
                ctx,
                button_disabled,
                self.finished_benchmarks,
                self.total_benchmarks,
                &self.status_label,
                &self.experiment_result
            )}
            {include_cdn_js()}
            {render_footer()}
        </>
        }
    }
}

impl AppRoot {
    fn start_benchmarks(&mut self) {
        self.experiment_result = ExperimentResult::Running;
        self.disable_controls(true);
        self.initialize_benchmark_data();

        self.start_next_benchmark_or_send(None);
    }

    fn disable_controls(&mut self, disabled: bool) {
        self.button_disabled = disabled;
        self.input_disabled = disabled;
    }

    fn initialize_benchmark_data(&mut self) {
        self.benchmark_results = vec![];
        self.remaining_benchmarks = VecDeque::from(vec![
            BenchmarkType::PageSize,
            BenchmarkType::CacheSize,
            BenchmarkType::TlbSize,
            BenchmarkType::CacheAssociativity,
            BenchmarkType::SinglePerformance,
        ]);

        self.total_benchmarks = self.remaining_benchmarks.len();
    }

    fn start_next_benchmark_or_send(&mut self, ctx: Option<&Context<Self>>) {
        if let Some(benchmark) = self.remaining_benchmarks.pop_front() {
            self.update_status_and_progress(benchmark);
            self.bridge.send(BenchmarkInput {
                page_origin: get_page_origin(),
                benchmark,
            });
        } else if let Some(ctx) = ctx {
            self.send_result(ctx);
        }
    }

    fn update_status_and_progress(&mut self, benchmark: BenchmarkType) {
        self.status_label = format!("Running: {}", benchmark);
        self.finished_benchmarks += 1;
    }

    fn handle_benchmark_complete(&mut self, ctx: &Context<Self>, result: BenchmarkResult) {
        self.benchmark_results.push(result);
        self.start_next_benchmark_or_send(Some(ctx));
    }

    fn send_result(&mut self, ctx: &Context<Self>) {
        let (results, times) = self.parse_results();

        let result = ResultDTO {
            model: self.model_input.clone(),
            user_agent: get_user_agent().unwrap_or_else(|| "unknown".to_string()),
            benchmark_results: results,
            times,
        };

        let link = ctx.link().clone();

        self.status_label = String::from("Uploading results...");

        wasm_bindgen_futures::spawn_local(async move {
            let response = Request::post("/api/result/upload")
                .json(&result)
                .unwrap()
                .send()
                .await
                .unwrap();

            link.send_message(AppRootMessage::BenchmarksFinished(
                response.status(),
                response.status_text(),
            ));
        });
    }

    fn handle_benchmarks_finished(&mut self, status: u16, status_text: String) {
        self.disable_controls(false);

        if status == 200 {
            // Success
            self.experiment_result = ExperimentResult::Success;
            self.status_label = String::from("Benchmarking finished");
        } else {
            // Something is wrong
            self.experiment_result = ExperimentResult::Error;
            self.status_label = format!("Error: {}. Please try again.", status_text);
        }
    }

    fn parse_results(&self) -> (Vec<Value>, Vec<f32>) {
        let mut results = vec![];
        let mut times = vec![];

        for result in self.benchmark_results.iter() {
            let value = serde_json::from_str::<Value>(result.result_json.clone().as_str()).unwrap();

            // TODO: Cloning the whole result JSON is not very optimal
            results.push(value);
            times.push(result.time);
        }

        (results, times)
    }
}

fn get_user_agent() -> Option<String> {
    let window = web_sys::window().expect("Missing window");
    let user_agent = window.navigator().user_agent();
    match user_agent {
        Ok(user_agent) => Some(user_agent),
        Err(_) => None,
    }
}

fn get_page_origin() -> String {
    let window = web_sys::window().expect("Missing window");
    window
        .location()
        .origin()
        .expect("Missing origin information")
}
