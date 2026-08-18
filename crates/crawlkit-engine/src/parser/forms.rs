//! Form structure extraction for input validation analysis.

use scraper::Html;

use super::selectors;
use super::HtmlParser;
use super::{ExtractedForm, ExtractedInput};

impl HtmlParser {
    // ---------------------------------------------------------------------------
    // Forms
    // ---------------------------------------------------------------------------
    pub(super) fn extract_forms(document: &Html) -> Vec<ExtractedForm> {
        let selector = selectors::form();

        let input_sel = selectors::input_select_textarea();

        let label_sel = selectors::label();

        document
            .select(selector)
            .map(|form| {
                let action = form.value().attr("action").map(String::from);
                let method = form.value().attr("method").unwrap_or("get").to_lowercase();

                let inputs: Vec<_> = form.select(input_sel).collect();
                let input_count = inputs.len();
                let has_file_input = inputs
                    .iter()
                    .any(|i| i.value().attr("type") == Some("file"));
                let has_search_input = inputs.iter().any(|i| {
                    i.value().attr("type") == Some("search")
                        || i.value().attr("role") == Some("search")
                });

                // Collect all label `for` targets within this form
                let label_for_ids: std::collections::HashSet<String> = form
                    .select(label_sel)
                    .filter_map(|l| l.value().attr("for").map(String::from))
                    .collect();

                // Collect all input nodes that are descendants of a <label>
                let inputs_in_labels: std::collections::HashSet<ego_tree::NodeId> = {
                    let inner_input_sel = selectors::input_select_textarea();
                    form.select(label_sel)
                        .flat_map(|label| label.select(inner_input_sel))
                        .map(|input| input.id())
                        .collect()
                };

                let extracted_inputs: Vec<ExtractedInput> = inputs
                    .iter()
                    .map(|input| {
                        let input_type = input.value().attr("type").map(String::from);
                        let name = input.value().attr("name").map(String::from);
                        let id = input.value().attr("id").map(String::from);
                        let aria_label = input.value().attr("aria-label").map(String::from);
                        let aria_labelledby =
                            input.value().attr("aria-labelledby").map(String::from);
                        let aria_describedby =
                            input.value().attr("aria-describedby").map(String::from);
                        let placeholder = input.value().attr("placeholder").map(String::from);
                        let required = input.value().attr("required").is_some()
                            || input.value().attr("aria-required") == Some("true");

                        let has_explicit_label = id
                            .as_ref()
                            .map(|id_val| label_for_ids.contains(id_val))
                            .unwrap_or(false);

                        let has_implicit_label = inputs_in_labels.contains(&input.id());

                        let has_label = has_explicit_label
                            || has_implicit_label
                            || aria_label.is_some()
                            || aria_labelledby.is_some();

                        ExtractedInput {
                            input_type,
                            name,
                            id,
                            has_label,
                            aria_label,
                            aria_labelledby,
                            aria_describedby,
                            placeholder,
                            required,
                        }
                    })
                    .collect();

                let has_fieldset = form.select(selectors::fieldset()).next().is_some();
                let has_legend = form.select(selectors::legend()).next().is_some();

                ExtractedForm {
                    action,
                    method,
                    input_count,
                    has_file_input,
                    has_search_input,
                    inputs: extracted_inputs,
                    has_fieldset,
                    has_legend,
                }
            })
            .collect()
    }
}
