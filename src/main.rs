use regex::Regex;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use serde_json::Value;
use std::fs;

mod def;
mod verify;

use def::*;
use verify::*;

fn parse_variable(value: String) -> (bool, u32, String)
/* mutability, indirection, name */ {
    let is_const = value.matches("const").count();
    let indirection = value.matches("*").count();

    let parts = value.split(" ");
    let name = parts.last();

    // mutability and indirection
    (is_const != 1, indirection as u32, name.unwrap().to_string())
}

fn parse_decl_stmt(state: &mut ExecutionState) {
    /* Pop DeclStmt */
    /*
    for k in 0..state.var_decl {
        /* Pop variable declaration */
        let value = state.cmd.pop().unwrap();

        /* Parse variable */
        let (mutable, indirection, label) = parse_variable(value);

        let ownership = true;

        let var = Variable {
            mutable,
            ownership,
            indirection,
        };
        state.variables.insert(label, var);
    }

    state.var_decl = 0;
    */

    // println!("{:?}", state.variables);
}

fn split(s: String) -> (String, String) {
    let (p1, p2) = s.split_once(":").unwrap();
    let label = p1.trim();
    let value = p2.trim();
    (label.to_string(), value.to_string())
}

fn post_ParmVarDecl(state: &mut ExecutionState, map: &serde_json::Map<std::string::String, Value>) {
    /* Parse ownership */
    let name = map.get("name").unwrap().as_str().unwrap().to_string();
    let qual_type = get_qual_type(map.get("type").unwrap()).unwrap();

    let ownership = state.annotation.contains("MOVE");
    state.annotation.clear();

    // println!("{} {}", name, qual_type);

    let is_const = qual_type.matches("const").count();
    let indirection = qual_type.matches("*").count();

    let variable = Variable {
        mutable: is_const == 0,
        ownership,
        indirection: indirection as u32,
    };

    let inst = Inst::ParamDecl(0, name, variable);
    state.inst.push(inst);
}

fn post_FieldDecl(state: &mut ExecutionState, map: &serde_json::Map<std::string::String, Value>) {
    /* Parse ownership */
    assert!(
        state.annotation.len() != 0,
        "FieldDecl ownership must be annotated!"
    );

    let ownership = state.annotation.contains("MOVE");
    state.annotation.clear();

    let name = map.get("name").unwrap().as_str().unwrap().to_string();
    let qual_type = get_qual_type(map.get("type").unwrap()).unwrap();

    println!("{} {}", name, qual_type);

    let is_const = qual_type.matches("const").count();
    let indirection = qual_type.matches("*").count();

    let variable = Variable {
        mutable: is_const == 0,
        ownership,
        indirection: indirection as u32,
    };

    let inst = Inst::FieldDecl(0, name, variable);
    state.inst.push(inst);
}

fn post_DeclRefExpr(state: &mut ExecutionState) {
    /* Pop DeclRefExpr */
    // let (l, v) = split(state.ast.pop().unwrap());

    // state.cmd.push(v);
}

fn parse_unexposed_expr(state: &mut ExecutionState) {
    /* Pop DeclRefExpr */
    // let (l, v) = split(state.ast.pop().unwrap());

    /* No side effects */
}

fn post_process_CallExpr(state: &mut ExecutionState, children: u32) {
    let mut args = Vec::new();
    for k in 0..children - 1 {
        let inst = state.inst.pop().unwrap();
        match inst {
            Inst::VarDecl(0, label, variable) => {
                args.push(ExprDescriptor::LocalVariable(label));
            }
            Inst::Eval(0, expr) => match expr {
                ExprDescriptor::FunctionCall(label, aargs) => {
                    args.push(ExprDescriptor::FunctionCall(label, aargs));
                }
                _ => {
                    unreachable!();
                }
            },
            _ => {
                unreachable!();
            }
        }
    }
    args.reverse();
    let func = state.inst.pop().unwrap();

    match func {
        Inst::VarDecl(0, label, variable) => {
            let inst = Inst::Eval(0, ExprDescriptor::FunctionCall(label, args));
            state.inst.push(inst);
        }
        _ => {
            unreachable!();
        }
    }
}

fn split_function_signature(signature: &str) -> Option<(String, String, String)> {
    // Define a regex to match the pattern
    let re = Regex::new(r"([^\(]*)(\([^\)]*\))(.+)").unwrap();

    // Apply the regex to the input string
    if let Some(captures) = re.captures(signature) {
        // Capture the three parts: before, the part in brackets, and after
        let part1 = captures.get(1)?.as_str().trim().to_string();
        let part2 = captures.get(2)?.as_str().trim().to_string();
        let part3 = captures.get(3)?.as_str().trim().to_string();
        Some((part1, part2, part3))
    } else {
        None
    }
}

fn remove_parentheses(s: &str) -> String {
    // Define a regular expression that matches everything inside parentheses (inclusive)
    let re = Regex::new(r"\([^)]*\)").unwrap();

    // Replace all matches with an empty string
    re.replace_all(s, "").to_string()
}

fn post_FunctionDecl(
    state: &mut ExecutionState,
    map: &serde_json::Map<std::string::String, Value>,
    inst_cnt: usize,
) {
    let mut name: Option<&str> = None;
    let mut qual_type: Option<&str> = None;

    let name = map.get("name").unwrap().as_str().unwrap().to_string();
    let qual_type = get_qual_type(map.get("type").unwrap()).unwrap();
    let line = map.get("loc").unwrap().get("line").unwrap().as_str();

    let ret_type = remove_parentheses(qual_type);
    let is_const = ret_type.matches("const").count();
    let indirection = ret_type.matches("*").count();

    let mut return_type = None;
    if ret_type.contains("void") && indirection == 0 {
    } else {
        return_type = Some(Variable {
            ownership: false,
            mutable: is_const == 0,
            indirection: indirection as u32,
        });
    }

    let curr_size = state.inst.len();

    let mut inst_set = Vec::new();
    for k in 0..curr_size - inst_cnt {
        let inst = state.inst.pop().unwrap();
        inst_set.push(inst);
    }
    inst_set.reverse();

    let mut inst = Vec::new();
    let mut param = Vec::new();
    for k in inst_set.iter() {
        match k {
            Inst::ParamDecl(line, name, property) => {
                param.push(property.clone());
            }
            Inst::InstSet(line, set) => {
                inst = set.clone();
            }
            _ => {
                unreachable!();
            }
        }
    }

    state
        .tl
        .sub_unit
        .push(TranslationUnitSet::Function(Function {
            name,
            param,
            ret_val: return_type,
            inst,
        }));
}

fn post_BinaryOperator(state: &mut ExecutionState, children: u32) {
    /* Pop BinaryOperator */
    // let _ = state.ast.pop();

    let rhs = state.inst.pop().unwrap();
    let lhs = state.inst.pop().unwrap();

    match (lhs, rhs) {
        (Inst::VarDecl(line, label, variable), Inst::Eval(line2, expr)) => {
            state.inst.push(Inst::Assign(0, label, expr));
        }
        _ => {
            unreachable!();
        }
    }
}

fn post_VarDecl(state: &mut ExecutionState) {
    /* Pop VarDecl */
    // let (_, value) = split(state.ast.pop().unwrap());

    // /* Push the raw string onto cmd for future process */
    // state.cmd.push(value);

    state.var_decl += 1;
}

fn post_attribute_annotate(state: &mut ExecutionState) {
    /* Pop annotation */
    // let (l, v) = split(state.ast.pop().unwrap());
    // state.annotation = v;
}

fn post_CompoundStmt(state: &mut ExecutionState, inst_cnt: usize) {
    let curr_size = state.inst.len();

    let mut inst_set = Vec::new();
    for k in 0..curr_size - inst_cnt {
        let inst = state.inst.pop().unwrap();
        inst_set.push(inst);
    }
    inst_set.reverse();

    state.inst.push(Inst::InstSet(0, inst_set));
}

fn post_IfStmt(
    state: &mut ExecutionState,
    map: &serde_json::Map<std::string::String, Value>,
    inst_cnt: usize,
) {
    let curr_size = state.inst.len();
    let mut inst_set = Vec::new();
    for k in 0..curr_size - inst_cnt {
        let inst = state.inst.pop().unwrap();
        inst_set.push(inst);
    }
    inst_set.reverse();

    // let range = map.get("range");
    // let begin = range.unwrap().get("begin").unwrap();
    // let line = begin.get("line").unwrap();
    let line = map
        .get("range")
        .unwrap()
        .get("begin")
        .unwrap()
        .get("line")
        .unwrap();
    state.inst.push(Inst::If(line.as_u64().unwrap_or_default(), inst_set));
}

fn post_ReturnStmt(state: &mut ExecutionState) {
    /* TODO: return real variable */
    state.inst.push(Inst::ReturnStmt(0, "".to_string()));
}

fn parse_paren_expr(state: &mut ExecutionState) {
    /* Throw away */
    // let _ = split(state.ast.pop().unwrap());
}

struct Component {}

// struct Declared impl Variable {}

// struct Declared Variable {
//     mutable: bool,
//     ownership: bool,
//     indirection: u32,
// }

// fn evaluate(expr: &ExprDescriptor) -> ExprResult {}

fn evaluate(rhs: &ExprDescriptor) -> ExprResult {
    match rhs {
        ExprDescriptor::FunctionCall(func, params) => {
            let mut args = Vec::new();
            for k in 0..params.len() {
                args.push(evaluate(&params[k]));
            }

            /* TODO: verify logic */

            return ExprResult::TemporaryVariable(/* todo */ true);
        }
        ExprDescriptor::LocalVariable(var) => {
            return ExprResult::DeclaredVariable(var.clone());
        }
    }
}

struct ExecutionState {
    // params: Option<Vec<Variable>>,
    // variables: HashMap<String, Variable>,
    // declared_functions: HashMap<String, Function>,
    depth: u32,
    // cmd: Vec<String>,
    annotation: String,
    var_decl: u32,
    inst: Vec<Inst>,
    tl: TranslationUnit,
}

fn post_processing(
    map: &serde_json::Map<std::string::String, Value>,
    state: &mut ExecutionState,
    children: u32,
    inst_cnt: usize,
) {
    if let Some(kind) = map.get("kind") {
        if let Some(kind_str) = kind.as_str() {
            match kind_str {
                "FunctionDecl" => {
                    post_FunctionDecl(state, map, inst_cnt);
                }
                "CallExpr" => {
                    post_process_CallExpr(state, children);
                }
                "UnexposedExpr" => {
                    parse_unexposed_expr(state);
                }
                "DeclRefExpr" => {
                    post_DeclRefExpr(state);
                }
                "ParmVarDecl" => {
                    post_ParmVarDecl(state, map);
                }
                "FieldDecl" => {
                    post_FieldDecl(state, map);
                }
                "DeclStmt" => {
                    parse_decl_stmt(state);
                }
                "BinaryOperator" => {
                    post_BinaryOperator(state, children);
                }
                "VarDecl" => {
                    post_VarDecl(state);
                }
                "attribute(annotate)" => {
                    post_attribute_annotate(state);
                }
                "CompoundStmt" => {
                    post_CompoundStmt(state, inst_cnt);
                }
                "ParenExpr" => {
                    parse_paren_expr(state);
                }
                "IfStmt" => {
                    post_IfStmt(state, map, inst_cnt);
                }
                "ReturnStmt" => {
                    post_ReturnStmt(state);
                }
                "BuiltinType" => {}
                "TypedefDecl" => {}
                "RecordDecl" => {}
                "RecordType" => {}
                "PointerType" => {}
                "ConstantArrayType" => {}
                // "ReturnStmt" => {}
                "AnnotateAttr" => {}
                "ImplicitCastExpr" => {}
                "TranslationUnitDecl" => {}
                "IntegerLiteral" => {}
                _ => {
                    println!("{}", kind_str);
                    todo!()
                }
            }
        }
    }
}

fn get_qual_type(value: &Value) -> Option<&str> {
    if let Value::Object(map) = value {
        if let Some(kind) = map.get("qualType") {
            if let Some(kind_str) = kind.as_str() {
                return Some(kind_str);
            }
        }
    }
    None
}

fn pre_process_referenced_decl(state: &mut ExecutionState, value: &Value) {
    if let Value::Object(map) = value {
        let mut name: Option<&str> = None;
        let mut qual_type: Option<&str> = None;
        let mut inner: Option<&Value> = None;

        // Traverse nested objects or arrays
        for (l, v) in map {
            match l.as_str() {
                "name" => name = v.as_str(),
                "type" => {
                    qual_type = get_qual_type(v);
                }
                "inner" => {
                    inner = Some(v);
                    break;
                }
                _ => {}
            }
        }

        // let stmt = format!("{}", name.unwrap());
        // state.cmd.push(stmt);

        // TODO:
        let variable = Variable {
            mutable: false,
            ownership: false,
            indirection: 0,
        };

        let inst = Inst::VarDecl(0, name.unwrap().to_string(), variable);
        state.inst.push(inst);
    }
}

fn extract_annotation_from_source(line: u64, start: u64, end: u64) -> Option<String> {
    // Open the file
    let path = Path::new("dummy.c");
    let file = File::open(&path).expect("Failed to open dummy.c");

    // Use BufReader to read the file line by line
    let reader = io::BufReader::new(file);

    // Find the specified line (note: line numbers are 1-based)
    let line_content = reader.lines().nth((line - 1) as usize);

    match line_content {
        Some(Ok(line_text)) => {
            // Extract the part of the line between start and end positions
            let line_len = line_text.len() as u64;
            if start < line_len && end <= line_len && start <= end {
                let rv = line_text[start as usize..end as usize].to_string();

                let rv = rv.as_str();

                let start = rv.find('\"')? + 1; // First double quote after `annotate(`
                let end = rv.rfind('\"')?; // Last double quote

                // Extract the substring between the quotes
                return Some(rv[start..end].to_string());
            } else {
                return None;
            }
        }
        _ => return None,
        None => return None,
    }
}

fn parse_annotation(state: &mut ExecutionState, value: &Value) -> Option<String> {
    if let Value::Object(map) = value {
        let l0 = map.get("begin")?.get("spellingLoc")?.get("line")?;

        let c0 = map.get("begin")?.get("spellingLoc")?.get("col")?;

        let l1 = map.get("end")?.get("spellingLoc")?.get("line")?;

        let c1 = map.get("end")?.get("spellingLoc")?.get("col")?;

        assert!(l0.as_u64() == l1.as_u64());
        let annotation =
            extract_annotation_from_source(l0.as_u64()?, c0.as_u64()? - 1, c1.as_u64()?);
        return annotation;
    }

    None
}

fn pre_processing(state: &mut ExecutionState, map: &serde_json::Map<std::string::String, Value>) {
    let mut kind: Option<&str> = None;
    let mut name: Option<&str> = None;
    let mut qual_type: Option<&str> = None;
    let mut inner: Option<&Value> = None;
    let mut range: Option<&Value> = None;
    let mut referenced_decl: Option<&Value> = None;

    // Traverse nested objects or arrays
    for (l, v) in map {
        match l.as_str() {
            "id" => { /* don't care */ }
            "loc" => { /* don't care */ }
            "range" => {
                range = Some(v);
            }
            "isUsed" => { /* don't care */ }
            "kind" => kind = v.as_str(),
            "name" => name = v.as_str(),
            "type" => {
                qual_type = get_qual_type(v);
                // todo!()
            }
            "inner" => {
                inner = Some(v);
                break;
            }
            "referencedDecl" => {
                referenced_decl = Some(v);
                break;
            }
            _ => {}
        }
    }

    match kind.unwrap_or("") {
        "VarDecl" => {
            // let push = format!("{} {}", qual_type.unwrap_or(""), name.unwrap_or(""));
            // state.cmd.push(push);

            /* TODO */
            let var = Variable {
                mutable: false,
                ownership: false,
                indirection: 0,
            };

            state
                .inst
                .push(Inst::VarDecl(0, name.unwrap_or("").to_string(), var));
        }
        "DeclStmt" => {}
        "TypedefDecl" => {
            return; /* Don't care */
        }
        "CallExpr" => {}
        "DeclRefExpr" => {
            if referenced_decl != None {
                pre_process_referenced_decl(state, referenced_decl.unwrap());
            }
        }
        "ParmVarDecl" => {
            // let push = format!("{} {}", qual_type.unwrap_or(""), name.unwrap_or(""));
            // state.cmd.push(push);
        }
        "FunctionDecl" => {
            // let push = format!("{} {}", qual_type.unwrap_or(""), name.unwrap_or(""));
            // state.cmd.push(push);
        }
        "CompoundStmt" => {}
        _ => {}
    }

    if kind.unwrap_or("") == "AnnotateAttr" {
        if range != None {
            let annotation = parse_annotation(state, range.unwrap());
            if annotation != None {
                state.annotation = annotation.unwrap();
            }
        }
    }

    let indent = "  ".repeat(state.depth as usize);
    println!("{}{}: {}", indent, kind.unwrap_or(""), name.unwrap_or(""));
}

fn traverse_json(state: &mut ExecutionState, value: &Value) -> u32 {
    state.depth += 1;
    let mut children = 0;
    if let Value::Object(map) = value {
        pre_processing(state, map);

        let inst_count = state.inst.len();

        if let Some(inner) = map.get("inner") {
            children = traverse_json(state, inner);
        }

        post_processing(map, state, children, inst_count);
    } else if let Value::Array(arr) = value {
        for val in arr {
            children += 1;
            traverse_json(state, val);
        }
    }
    state.depth -= 1;

    children
}

fn main() {
    // Read the contents of the JSON file
    let file_path = "dummy.json";
    let json_content = fs::read_to_string(file_path).expect("Failed to read file");

    // Parse the JSON content into a serde_json::Value
    let parsed_json: Value = serde_json::from_str(&json_content).expect("Failed to parse JSON");

    let mut state = ExecutionState {
        // params: None,
        depth: 0,
        // cmd: Vec::new(),
        annotation: String::new(),
        // declared_functions: HashMap::new(),
        // variables: HashMap::new(),
        var_decl: 0,
        inst: Vec::new(),
        tl: TranslationUnit {
            sub_unit: Vec::new(),
        },
    };

    // Start recursive traversal
    traverse_json(&mut state, &parsed_json);

    verify(&state.tl);

    println!("Completed!");
}

/*
 *  Test #1: a = b
 *  Test #2: *a = *b
 *  Test #3: a = func(a);
 *  Test #4: a = func(a, b);
 *  Test #4: ? = func(a, b);
 *
 */
