use itertools::Itertools;
use lsp_types::{
	Documentation, MarkupContent, MarkupKind, ParameterInformation, ParameterLabel, Position, SignatureHelp,
	SignatureInformation,
};

use crate::ast::{Expr, ExprKind, NewExpr, Symbol};
use crate::docs::Documented;
use crate::lsp::sync::FILES;

use crate::type_check::{resolve_user_defined_type, CLASS_INIT_NAME};
use crate::visit::{visit_expr, Visit};
use crate::wasm_util::{ptr_to_string, string_to_combined_ptr, WASM_RETURN_ERROR};

#[no_mangle]
pub unsafe extern "C" fn wingc_on_signature_help(ptr: u32, len: u32) -> u64 {
	let parse_string = ptr_to_string(ptr, len);
	if let Ok(parsed) = serde_json::from_str(&parse_string) {
		let result = on_signature_help(parsed);
		let result = serde_json::to_string(&result).unwrap();

		// return result as u64 with ptr and len
		string_to_combined_ptr(result)
	} else {
		WASM_RETURN_ERROR
	}
}

pub fn on_signature_help(params: lsp_types::SignatureHelpParams) -> Option<SignatureHelp> {
	FILES.with(|files| {
		let files = files.borrow();
		let uri = params.text_document_position_params.text_document.uri;
		let file_data = files.get(&uri).expect("File must be open to get completions");

		let root_scope = &file_data.scope;

		let mut scope_visitor = ScopeVisitor::new(params.text_document_position_params.position);
		scope_visitor.visit_scope(root_scope);
		let expr = scope_visitor.call_expr?;

		let sig_data: (
			crate::type_check::UnsafeRef<crate::type_check::Type>,
			&crate::ast::ArgList,
		) = match &expr.kind {
			ExprKind::New(new_expr) => {
				let NewExpr { class, arg_list, .. } = new_expr;
				let Some(udt) = class.as_type_reference() else {
					return None;
				};

				let Some(t) = resolve_user_defined_type(&udt, root_scope.env.borrow().as_ref()?, 0).ok() else {
					return None;
				};

				let init_lookup = t.as_class()?.env.lookup(
					&Symbol {
						name: CLASS_INIT_NAME.into(),
						span: Default::default(),
					},
					None,
				);

				(init_lookup?.as_variable()?.type_, arg_list)
			}
			ExprKind::Call { callee, arg_list } => {
				let t = file_data.types.get_expr_type(callee);
				(t, arg_list)
			}
			_ => return None,
		};

		let sig = sig_data.0.as_function_sig()?;
		let provided_args = sig_data.1;

		let positional_arg_pos = provided_args
			.pos_args
			.iter()
			.enumerate()
			.filter(|(_, arg)| params.text_document_position_params.position <= arg.span.end.into())
			.count();
		let named_arg_pos = provided_args
			.named_args
			.iter()
			.find(|arg| arg.1.span.contains(&params.text_document_position_params.position));

		let param_data = sig
			.parameters
			.iter()
			.enumerate()
			.map(|p| {
				if p.0 == sig.parameters.len() - 1 && p.1.typeref.maybe_unwrap_option().is_struct() {
					format!("...{}", p.1.name)
				} else {
					format!("{}: {}", p.1.name, p.1.typeref)
				}
			})
			.collect_vec();

		let active_parameter = if named_arg_pos.is_some() {
			sig.parameters.len() - 1
		} else {
			provided_args.pos_args.len() - positional_arg_pos
		}
		.min(param_data.len() - 1)
		.max(0);

		let param_text = param_data.join(", ");
		let label = format!("({}): {}", param_text, sig.return_type);

		let signature_info = SignatureInformation {
			label,
			documentation: Some(Documentation::MarkupContent(MarkupContent {
				kind: MarkupKind::Markdown,
				value: sig_data.0.render_docs(),
			})),
			parameters: Some(
				sig
					.parameters
					.iter()
					.enumerate()
					.map(|p| {
						let last_arg = p.0 == sig.parameters.len() - 1;
						let p_type = p.1.typeref;
						let structy = p_type.maybe_unwrap_option();
						let structy = structy.as_struct();
						let p_docs = p_type.render_docs();
						let p_docs = if p_docs.is_empty() {
							None
						} else {
							Some(Documentation::MarkupContent(MarkupContent {
								kind: MarkupKind::Markdown,
								value: p_docs,
							}))
						};
						ParameterInformation {
							label: if last_arg && structy.is_some() {
								ParameterLabel::Simple(format!("...{}", p.1.name))
							} else {
								ParameterLabel::Simple(param_data.get(p.0).unwrap_or(&format!("{}: {}", p.0, p_type)).clone())
							},
							documentation: if structy.is_some() {
								//check if this is the last arg, allowing for expansion syntax
								if p.0 == sig.parameters.len() - 1 {
									Some(Documentation::MarkupContent(MarkupContent {
										kind: MarkupKind::Markdown,
										value: p_type.render_docs(),
									}))
								} else {
									p_docs
								}
							} else {
								p_docs
							},
						}
					})
					.collect(),
			),

			active_parameter: Some(active_parameter as u32),
		};

		Some(SignatureHelp {
			signatures: vec![signature_info],
			active_signature: None,
			active_parameter: None,
		})
	})
}

/// This visitor is used to find the scope
/// and relevant expression that contains a given location.
pub struct ScopeVisitor<'a> {
	/// The target location we're looking for
	pub location: Position,
	/// The nearest expression before (or containing) the target location
	pub call_expr: Option<&'a Expr>,
}

impl<'a> ScopeVisitor<'a> {
	pub fn new(location: Position) -> Self {
		Self {
			location,
			call_expr: None,
		}
	}
}

impl<'a> Visit<'a> for ScopeVisitor<'a> {
	fn visit_expr(&mut self, node: &'a Expr) {
		if self.call_expr.is_some() {
			return;
		}

		if node.span.contains(&self.location) {
			match node.kind {
				ExprKind::Call { .. } | ExprKind::New { .. } => {
					self.call_expr = Some(node);
				}
				_ => {}
			}
		}

		visit_expr(self, node);
	}
}

#[cfg(test)]
mod tests {
	use crate::lsp::signature::*;
	use crate::lsp::sync::test_utils::*;
	use lsp_types::*;

	/// Creates a snapshot test for a given wing program's signature at a given position
	/// In the wing program, place a comment "//^" into the text where the "^" is pointing to the desired character position
	///
	/// First parameter will be the name of the tests, as well as the identifier to use for the list of completion in the asserts (see last parameter)
	/// Second parameter is the wing code block as a string literal
	/// After the first two parameters, any additional are optional statements that should be used for asserting on the given completions.
	///
	/// Result is a list of [CompletionItem]s
	macro_rules! test_signature {
		($name:ident, $code:literal, $($assertion:stmt)*) => {
			#[test]
			fn $name() {
				let text_document_position = load_file_with_contents($code);
				let sig = on_signature_help(SignatureHelpParams {
					context: None,
					text_document_position_params: TextDocumentPositionParams {
						text_document: text_document_position.text_document.clone(),
						position: Position {
							line: text_document_position.position.line,
							character: text_document_position.position.character,
						},
					},
					work_done_progress_params: Default::default(),
				});

				if let Some($name) = sig {
					insta::with_settings!(
						{
							prepend_module_to_snapshot => false,
							omit_expression => true,
							snapshot_path => "./snapshots/signature",
						}, {
							insta::assert_yaml_snapshot!($name);
						}
					);
					$($assertion)*
				}
			}
		};
	}

	test_signature!(empty, "", assert!(false));

	test_signature!(
		second_arg_active,
		r#"
bring cloud;
let bucket = new cloud.Bucket();
bucket.addObject("key", )
                     //^
		"#,
	);

	test_signature!(
		named_arg_active,
		r#"
bring cloud;
let bucket = new cloud.Bucket();
bucket.onEvent(inflight () => {
	bucket.delete("", );
                 //^
});"#,
	);

	test_signature!(
		constructor_arg,
		r#"
bring cloud;
let bucket = new cloud.Bucket( );
                            //^"#,
	);
}
