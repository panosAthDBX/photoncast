//! Calculator methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    pub(super) fn calculator_result_to_search_result(result: &CalculatorResult) -> SearchResult {
        let subtitle = Self::calculator_subtitle(result);
        SearchResult {
            id: photoncast_core::search::SearchResultId::new(format!(
                "calculator:{}",
                result.expression
            )),
            title: result.formatted_value.clone(),
            subtitle,
            icon: IconSource::Emoji {
                char: Self::calculator_icon(result),
            },
            result_type: CoreResultType::SystemCommand,
            score: 0.0,
            match_indices: vec![],
            requires_permissions: false,
            action: SearchAction::CopyToClipboard {
                text: result.formatted_value.clone(),
            },
        }
    }

    pub(super) fn calculator_result_to_result_item(result: &CalculatorResult) -> ResultItem {
        ResultItem {
            id: SharedString::from(format!("calculator:{}", result.expression)),
            title: result.formatted_value.clone().into(),
            subtitle: Self::calculator_subtitle(result).into(),
            icon_emoji: SharedString::from(Self::calculator_icon(result).to_string()),
            icon_path: None,
            result_type: ResultType::Calculator,
            bundle_id: None,
            app_path: None,
            requires_permissions: false,
        }
    }

    pub(super) fn calculator_subtitle(result: &CalculatorResult) -> String {
        result
            .details
            .clone()
            .unwrap_or_else(|| result.expression.clone())
    }

    pub(super) fn schedule_calculator_evaluation(&mut self, cx: &mut ViewContext<Self>) {
        let expression = self.search.query.to_string();

        if !is_calculator_expression(&expression) {
            if self.calculator.result.is_some() {
                self.calculator.result = None;
                self.rebuild_results(cx);
            }
            self.calculator.generation = self.calculator.generation.saturating_add(1);
            return;
        }

        self.calculator.generation = self.calculator.generation.saturating_add(1);
        let generation = self.calculator.generation;
        let calculator_command = Arc::clone(&self.calculator.command);
        let calculator_runtime = Arc::clone(&self.calculator.runtime);

        cx.spawn(|this, mut cx| async move {
            cx.background_executor()
                .timer(Duration::from_millis(120))
                .await;

            let should_eval = this
                .update(&mut cx, |view, _| view.calculator.generation == generation)
                .unwrap_or(false);

            if !should_eval {
                return;
            }

            let expression_clone = expression.clone();
            let evaluation = cx
                .background_executor()
                .spawn(async move {
                    let mut command = calculator_command.write();
                    if !command.is_ready() {
                        calculator_runtime
                            .block_on(command.initialize())
                            .map_err(|err| err.to_string())?;
                    }

                    calculator_runtime
                        .block_on(command.evaluate(&expression_clone))
                        .map_err(|err| err.to_string())
                })
                .await;

            let _ = this.update(&mut cx, |view, cx| {
                if view.calculator.generation != generation {
                    return;
                }

                match evaluation {
                    Ok(result) => {
                        view.calculator.result = Some(result);
                    },
                    Err(error) => {
                        tracing::warn!("Calculator evaluation failed: {}", error);
                        view.calculator.result = None;
                    },
                }

                view.rebuild_results(cx);
                cx.notify();
            });
        })
        .detach();
    }
}
