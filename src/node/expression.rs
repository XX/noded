use std::collections::HashMap;

use egui::{Color32, Ui};
use egui_snarl::ui::{PinInfo, WireStyle};
use egui_snarl::{InPin, InPinId, Snarl};

use super::viewer::{NUMBER_COLOR, STRING_COLOR, format_float};
use super::{Node, NodeFlags};

/// Node for evaluating algebraic expression
/// It has number of inputs equal to number of variables in the expression.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ExpressionNode {
    pub text: String,
    pub bindings: Vec<String>,
    pub values: Vec<f64>,
    pub expr: Expression,
}

impl ExpressionNode {
    pub const NAME: &str = "Expression";
    pub const INPUTS: [u64; 0] = [];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::NUMBER.bits() | NodeFlags::EXPRESSION.bits()];

    pub fn new() -> Self {
        Self {
            text: "0".to_string(),
            bindings: Vec::new(),
            values: Vec::new(),
            expr: Expression::Val(0.0),
        }
    }

    pub fn eval(&self) -> f64 {
        self.expr.eval(&self.bindings, &self.values)
    }

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                let changed = match &*pin.remotes {
                    [] => {
                        let input = snarl[pin.id.node].string_in();
                        let response = egui::TextEdit::singleline(input)
                            .clip_text(false)
                            .desired_width(0.0)
                            .margin(ui.spacing().item_spacing)
                            .show(ui)
                            .response;

                        response.changed()
                    },
                    [remote] => {
                        let new_string = snarl[remote.node].string_out().to_owned();

                        egui::TextEdit::singleline(&mut &*new_string)
                            .clip_text(false)
                            .desired_width(0.0)
                            .margin(ui.spacing().item_spacing)
                            .show(ui);

                        let input = snarl[pin.id.node].string_in();
                        if new_string == *input {
                            false
                        } else {
                            *input = new_string;
                            true
                        }
                    },
                    _ => unreachable!("Expr pins has only one wire"),
                };

                if changed {
                    let node = snarl[pin.id.node].as_expression_mut();

                    if let Ok(expr) = syn::parse_str(&node.text) {
                        node.expr = expr;

                        let values =
                            Iterator::zip(node.bindings.iter().map(String::clone), node.values.iter().copied())
                                .collect::<HashMap<String, f64>>();

                        let mut new_bindings = Vec::new();
                        node.expr.extend_bindings(&mut new_bindings);

                        let old_bindings = std::mem::replace(&mut node.bindings, new_bindings.clone());

                        let new_values = new_bindings
                            .iter()
                            .map(|name| values.get(&**name).copied().unwrap_or(0.0))
                            .collect::<Vec<_>>();

                        node.values = new_values;

                        let old_inputs = (0..old_bindings.len())
                            .map(|idx| {
                                snarl.in_pin(InPinId {
                                    node: pin.id.node,
                                    input: idx + 1,
                                })
                            })
                            .collect::<Vec<_>>();

                        for (idx, name) in old_bindings.iter().enumerate() {
                            let new_idx = new_bindings.iter().position(|new_name| *new_name == *name);

                            match new_idx {
                                None => {
                                    snarl.drop_inputs(old_inputs[idx].id);
                                },
                                Some(new_idx) if new_idx != idx => {
                                    let new_in_pin = InPinId {
                                        node: pin.id.node,
                                        input: new_idx,
                                    };
                                    for &remote in &old_inputs[idx].remotes {
                                        snarl.disconnect(remote, old_inputs[idx].id);
                                        snarl.connect(remote, new_in_pin);
                                    }
                                },
                                _ => {},
                            }
                        }
                    }
                }
                PinInfo::circle()
                    .with_fill(STRING_COLOR)
                    .with_wire_style(WireStyle::AxisAligned { corner_radius: 10.0 })
            },
            idx => {
                if idx <= snarl[pin.id.node].as_expression_mut().bindings.len() {
                    match &*pin.remotes {
                        [] => {
                            let node = snarl[pin.id.node].as_expression_mut();
                            ui.label(&node.bindings[idx - 1]);
                            ui.add(egui::DragValue::new(&mut node.values[idx - 1]));
                            PinInfo::circle().with_fill(NUMBER_COLOR)
                        },
                        [remote] => {
                            let new_value = snarl[remote.node].number_out();
                            let node = snarl[pin.id.node].as_expression_mut();
                            ui.label(&node.bindings[idx - 1]);
                            ui.label(format_float(new_value));
                            node.values[idx - 1] = new_value;
                            PinInfo::circle().with_fill(NUMBER_COLOR)
                        },
                        _ => unreachable!("Expr pins has only one wire"),
                    }
                } else {
                    ui.label("Removed");
                    PinInfo::circle().with_fill(Color32::BLACK)
                }
            },
        }
    }

    pub fn disconnect_input(&mut self, _input: usize) {}
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum UnOp {
    Pos,
    Neg,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Expression {
    Var(String),
    Val(f64),
    UnOp {
        op: UnOp,
        expr: Box<Expression>,
    },
    BinOp {
        lhs: Box<Expression>,
        op: BinOp,
        rhs: Box<Expression>,
    },
}

impl Expression {
    pub fn eval(&self, bindings: &[String], args: &[f64]) -> f64 {
        let binding_index = |name: &str| bindings.iter().position(|binding| binding == name).unwrap();

        match self {
            Expression::Var(name) => args[binding_index(name)],
            Expression::Val(value) => *value,
            Expression::UnOp { op, expr } => match op {
                UnOp::Pos => expr.eval(bindings, args),
                UnOp::Neg => -expr.eval(bindings, args),
            },
            Expression::BinOp { lhs, op, rhs } => match op {
                BinOp::Add => lhs.eval(bindings, args) + rhs.eval(bindings, args),
                BinOp::Sub => lhs.eval(bindings, args) - rhs.eval(bindings, args),
                BinOp::Mul => lhs.eval(bindings, args) * rhs.eval(bindings, args),
                BinOp::Div => lhs.eval(bindings, args) / rhs.eval(bindings, args),
            },
        }
    }

    pub fn extend_bindings(&self, bindings: &mut Vec<String>) {
        match self {
            Expression::Var(name) => {
                if !bindings.contains(name) {
                    bindings.push(name.clone());
                }
            },
            Expression::Val(_) => {},
            Expression::UnOp { expr, .. } => {
                expr.extend_bindings(bindings);
            },
            Expression::BinOp { lhs, rhs, .. } => {
                lhs.extend_bindings(bindings);
                rhs.extend_bindings(bindings);
            },
        }
    }
}

impl syn::parse::Parse for UnOp {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            Ok(UnOp::Pos)
        } else if lookahead.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            Ok(UnOp::Neg)
        } else {
            Err(lookahead.error())
        }
    }
}

impl syn::parse::Parse for BinOp {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            Ok(BinOp::Add)
        } else if lookahead.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            Ok(BinOp::Sub)
        } else if lookahead.peek(syn::Token![*]) {
            input.parse::<syn::Token![*]>()?;
            Ok(BinOp::Mul)
        } else if lookahead.peek(syn::Token![/]) {
            input.parse::<syn::Token![/]>()?;
            Ok(BinOp::Div)
        } else {
            Err(lookahead.error())
        }
    }
}

impl syn::parse::Parse for Expression {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let lhs;
        if lookahead.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let expr = content.parse::<Expression>()?;
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        // } else if lookahead.peek(syn::LitFloat) {
        //     let lit = input.parse::<syn::LitFloat>()?;
        //     let value = lit.base10_parse::<f64>()?;
        //     let expr = Expr::Val(value);
        //     if input.is_empty() {
        //         return Ok(expr);
        //     }
        //     lhs = expr;
        } else if lookahead.peek(syn::LitInt) {
            let lit = input.parse::<syn::LitInt>()?;
            let value = lit.base10_parse::<f64>()?;
            let expr = Expression::Val(value);
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            let expr = Expression::Var(ident.to_string());
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else {
            let unop = input.parse::<UnOp>()?;

            return Self::parse_with_unop(unop, input);
        }

        let binop = input.parse::<BinOp>()?;

        Self::parse_binop(Box::new(lhs), binop, input)
    }
}

impl Expression {
    fn parse_with_unop(op: UnOp, input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let lhs;
        if lookahead.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let expr = Expression::UnOp {
                op,
                expr: Box::new(content.parse::<Expression>()?),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::LitFloat) {
            let lit = input.parse::<syn::LitFloat>()?;
            let value = lit.base10_parse::<f64>()?;
            let expr = Expression::UnOp {
                op,
                expr: Box::new(Expression::Val(value)),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::LitInt) {
            let lit = input.parse::<syn::LitInt>()?;
            let value = lit.base10_parse::<f64>()?;
            let expr = Expression::UnOp {
                op,
                expr: Box::new(Expression::Val(value)),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            let expr = Expression::UnOp {
                op,
                expr: Box::new(Expression::Var(ident.to_string())),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else {
            return Err(lookahead.error());
        }

        let op = input.parse::<BinOp>()?;

        Self::parse_binop(Box::new(lhs), op, input)
    }

    fn parse_binop(lhs: Box<Expression>, op: BinOp, input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let rhs;
        if lookahead.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            rhs = Box::new(content.parse::<Expression>()?);
            if input.is_empty() {
                return Ok(Expression::BinOp { lhs, op, rhs });
            }
        } else if lookahead.peek(syn::LitFloat) {
            let lit = input.parse::<syn::LitFloat>()?;
            let value = lit.base10_parse::<f64>()?;
            rhs = Box::new(Expression::Val(value));
            if input.is_empty() {
                return Ok(Expression::BinOp { lhs, op, rhs });
            }
        } else if lookahead.peek(syn::LitInt) {
            let lit = input.parse::<syn::LitInt>()?;
            let value = lit.base10_parse::<f64>()?;
            rhs = Box::new(Expression::Val(value));
            if input.is_empty() {
                return Ok(Expression::BinOp { lhs, op, rhs });
            }
        } else if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            rhs = Box::new(Expression::Var(ident.to_string()));
            if input.is_empty() {
                return Ok(Expression::BinOp { lhs, op, rhs });
            }
        } else {
            return Err(lookahead.error());
        }

        let next_op = input.parse::<BinOp>()?;

        if let (BinOp::Add | BinOp::Sub, BinOp::Mul | BinOp::Div) = (op, next_op) {
            let rhs = Self::parse_binop(rhs, next_op, input)?;
            Ok(Self::BinOp {
                lhs,
                op,
                rhs: Box::new(rhs),
            })
        } else {
            let lhs = Self::BinOp { lhs, op, rhs };
            Self::parse_binop(Box::new(lhs), next_op, input)
        }
    }
}
