
use crate::def::*;

use std::collections::HashMap;

// use crate::TranslationUnitSet::Function;

struct State {
    variables: HashMap<String, Variable>,
    functions: HashMap<String, Function>,
    is_forked: bool,
}

fn eval(state: &State, expr: &ExprDescriptor) -> Option<Variable> {
    match expr {
        ExprDescriptor::FunctionCall(name, args) => {
            println!("### eval: Function");
            let mut vars = Vec::new();
            for arg in args.iter() {
                vars.push(eval(state, arg));
            }

            /* TODO: borrow-checker to verify variables against function parameter list */

            println!("### eval: name = {}", name);
            state.functions.get(name)?.ret_val
        }
        ExprDescriptor::LocalVariable(name) => {
            println!("### eval: Variable name = {}", name);
            Some(state.variables.get(name).unwrap().clone())
        }
    }
}

enum ExitCode {
    EarlyExit,
}

use std::{
    process::{exit, Command},
    thread::sleep,
    time::Duration,
};

use nix::{
    sys::wait::waitpid,
    unistd::{fork, write, ForkResult},
};

fn process_inst(state: &mut State, inst: &Inst) -> Result<i32, ExitCode> {
    match inst {
        Inst::InstSet(inst_set) => {
            // Allow early exit
            let _ = process(state, inst_set)?;
        }
        Inst::ParamDecl(label, variable) => {
            state.variables.insert(label.to_string(), variable.clone());
        }
        Inst::VarDecl(label, variable) => {
            state.variables.insert(label.to_string(), variable.clone());
        }
        Inst::Assign(lhs, rhs) => {
            let rv = eval(state, rhs);

            /* TODO: borrow-checker to verify variables against function parameter list */

            state.variables.insert(lhs.to_string(), rv.unwrap());
        }
        Inst::Eval(rhs) => {
            let _ = eval(state, rhs);
        }
        Inst::If(inst_list) => match inst_list.len() {
            2 => {
                let eval = inst_list.get(0);
                let path = inst_list.get(1);
                unsafe {
                    match fork().unwrap() {
                        ForkResult::Child => {
                            /* execute the condition */
                            state.is_forked = true;
                            process_inst(state, path.unwrap())?;
                        }
                        ForkResult::Parent { child } => {
                            /* wait for pid */
                            waitpid(Some(child), None).unwrap();
                        }
                    }
                }
            }
            3 => {
                let eval = inst_list.get(0);
                let p1 = inst_list.get(1);
                let p2 = inst_list.get(2);
                /* TODO: enable both path */
                unsafe {
                    // match fork().unwrap() {
                    //     ForkResult::Child => {
                    //         /* execute the condition */
                    //         state.is_forked = true;
                    //         process_inst(state, p1.unwrap())?;
                    //     }
                    //     ForkResult::Parent { child } => {
                    //         /* wait for pid */
                    //         waitpid(Some(child), None).unwrap();
                    process_inst(state, p1.unwrap())?;
                    // }
                    // }
                }
            }
            _ => {
                unreachable!();
            }
        },
        Inst::ReturnStmt(_) => {
            return Err(ExitCode::EarlyExit);
        }
        Inst::FieldDecl(_, _) => todo!(),
    }

    Ok(0)
}

fn process(state: &mut State, inst_list: &Vec<Inst>) -> Result<i32, ExitCode> {
    for inst in inst_list {
        process_inst(state, inst)?;
    }

    Ok(0)
}

// fn count_if_inst(inst: &Inst) -> u32 {
//     match inst {
//         Inst::InstSet(inst_set) => {}

//         Inst::If(inst_list) => {}
//     }
// }

// fn count_if(inst_list: &Vec<Inst>) -> u32 {
//     let ifs = 0;
//     for inst in inst_list {
//         ifs += count_if_inst(inst)?;
//     }
//     ifs
// }

pub fn verify(tl: &TranslationUnit) {
    // let variables = HashMap::new();
    let mut state = State {
        variables: HashMap::new(),
        functions: HashMap::new(),
        is_forked: false,
    };
    for unit in tl.sub_unit.iter() {
        match unit {
            TranslationUnitSet::Function(function) => {
                // let ifs = count_if(&function.inst);
                let rv = process(&mut state, &function.inst);
                match rv {
                    Ok(_) => {
                        /* Insert into function map for later lookup */
                        state
                            .functions
                            .insert(function.name.clone(), function.clone());
                    }
                    Err(code) => match code {
                        ExitCode::EarlyExit => {
                            exit(0);
                        }
                    },
                }
            }
        }
    }
}