use std::fmt;

use super::{CastOpType, Expression, Variable};
use crate::prelude::*;

/// A side-effectful operation.
/// Can be a register assignment or a memory load/store operation.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub enum Def {
    /// A memory load into the register given by `var`.
    Load {
        /// The target register of the memory load.
        /// The size of `var` also determines the number of bytes read from memory.
        var: Variable,
        /// The expression computing the address from which to read from.
        /// The size of `address` is required to match the pointer size of the corresponding CPU architecture.
        address: Expression,
    },
    /// A memory store operation.
    Store {
        /// The expression computing the address that is written to.
        /// The size of `address` is required to match the pointer size of the corresponding CPU architecture.
        address: Expression,
        /// The expression computing the value that is written to memory.
        /// The size of `value` also determines the number of bytes written.
        value: Expression,
    },
    /// A register assignment, assigning the result of the expression `value` to the register `var`.
    Assign {
        /// The register that is written to.
        var: Variable,
        /// The expression computing the value that is assigned to the register.
        value: Expression,
    },
}

impl Term<Def> {
    /// This function checks whether the instruction
    /// is a zero extension of the overwritten sub register of the previous instruction.
    /// If so, returns its TID
    pub fn check_for_zero_extension(
        &self,
        output_name: String,
        output_sub_register: String,
    ) -> Option<Tid> {
        match &self.term {
            Def::Assign {
                var,
                value:
                    Expression::Cast {
                        op: CastOpType::IntZExt,
                        arg,
                        ..
                    },
            } if output_name == var.name => {
                let argument: &Expression = arg;
                match argument {
                    Expression::Var(var) if var.name == output_sub_register => {
                        Some(self.tid.clone())
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Substitute every occurence of `input_var` in the address and value expressions
    /// with `replace_with_expression`.
    /// Does not change the target variable of assignment- and load-instructions.
    pub fn substitute_input_var(
        &mut self,
        input_var: &Variable,
        replace_with_expression: &Expression,
    ) {
        match &mut self.term {
            Def::Assign { var: _, value } => {
                value.substitute_input_var(input_var, replace_with_expression)
            }
            Def::Load { var: _, address } => {
                address.substitute_input_var(input_var, replace_with_expression)
            }
            Def::Store { address, value } => {
                address.substitute_input_var(input_var, replace_with_expression);
                value.substitute_input_var(input_var, replace_with_expression);
            }
        }
    }
}

impl fmt::Display for Def {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Def::Load { var, address } => write!(f, "{var} := Load from {address}"),
            Def::Store { address, value } => write!(f, "Store at {address} := {value}"),
            Def::Assign { var, value } => write!(f, "{var} = {value}"),
        }
    }
}

impl Def {
    /// Returns all constants that appear in the def.
    pub fn referenced_constants(&self) -> Option<Vec<Bitvector>> {
        match self {
            Def::Load { address: expr, .. } | Def::Assign { value: expr, .. } => {
                expr.referenced_constants()
            }
            Def::Store {
                address: expr0,
                value: expr1,
            } => match (expr0.referenced_constants(), expr1.referenced_constants()) {
                (None, c) | (c, None) => c,
                (Some(mut c0), Some(c1)) => {
                    c0.extend(c1);
                    Some(c0)
                }
            },
        }
    }

    /// Returns the sum of the recursion depths of all expressions in this
    /// `Def`.
    pub fn recursion_depth(&self) -> u64 {
        match self {
            Def::Load { address: expr, .. } | Def::Assign { value: expr, .. } => {
                expr.recursion_depth()
            }
            Def::Store {
                address: expr1,
                value: expr2,
            } => expr1.recursion_depth() + expr2.recursion_depth(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{expr, intermediate_representation::*, variable};

    #[test]
    fn zero_extension_check() {
        let zero_extend_def = Term {
            tid: Tid::new("zero_tid"),
            term: Def::Assign {
                var: variable!("RAX:8"),
                value: Expression::Cast {
                    op: CastOpType::IntZExt,
                    size: ByteSize::new(8),
                    arg: Box::new(expr!("EAX:8")),
                },
            },
        };
        // An expression that is a zero extension but does not directly contain a variable
        let zero_extend_but_no_var_def = Term {
            tid: Tid::new("zero_tid"),
            term: Def::Assign {
                var: variable!("RAX:8"),
                value: Expression::Cast {
                    op: CastOpType::IntZExt,
                    size: ByteSize::new(8),
                    arg: Box::new(expr!("EAX:8 - ECX:8")),
                },
            },
        };

        let non_zero_extend_def = Term {
            tid: Tid::new("zero_tid"),
            term: Def::Assign {
                var: variable!("RAX:8"),
                value: Expression::Cast {
                    op: CastOpType::IntSExt,
                    size: ByteSize::new(8),
                    arg: Box::new(expr!("EAX:8")),
                },
            },
        };

        assert_eq!(
            zero_extend_def.check_for_zero_extension(String::from("RAX"), String::from("EAX")),
            Some(Tid::new("zero_tid"))
        );
        assert_eq!(
            zero_extend_but_no_var_def
                .check_for_zero_extension(String::from("RAX"), String::from("EAX")),
            None
        );
        assert_eq!(
            non_zero_extend_def.check_for_zero_extension(String::from("RAX"), String::from("EAX")),
            None
        );
    }
}
