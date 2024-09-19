
extern crate clang_sys;
use regex::Regex;

use std::collections::HashMap;

use clang_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::ptr;

fn pre_process(cursor: CXCursor, state: &mut ExecutionState) -> (String, String) {
    unsafe {
        let kind_spelling = clang_getCursorKindSpelling(clang_getCursorKind(cursor));
        let spelling = clang_getCursorSpelling(cursor);

        // Convert C strings to Rust strings
        let kind_cstr = CStr::from_ptr(clang_getCString(kind_spelling));
        let spelling_cstr = CStr::from_ptr(clang_getCString(spelling));

        let indentation = state.indent;

        // Print indentation
        for _ in 0..indentation {
            print!("  ");
        }

        let cursor_type = clang_getCursorType(cursor);
        if clang_isConstQualifiedType(cursor_type) != 0 {
            print!(" [const] ");
        }

        let node_type = kind_cstr.to_str().unwrap().to_string();
        let mut node_value = spelling_cstr.to_str().unwrap().to_string();

        let type_spelling = clang_getTypeSpelling(cursor_type);
        let type_spelling_csr = CStr::from_ptr(clang_getCString(type_spelling));
        // print!(" (Type: {})", type_spelling_csr.to_string_lossy());

        if node_type == "ParmDecl" || node_type == "VarDecl" || node_type == "FunctionDecl" {
            let nn = format!(
                "{} {}",
                type_spelling_csr.to_str().unwrap().to_string(),
                node_value
            );
            node_value = nn;
        }

        if node_type == "CallExpr" {
            state.decl_ref_expr_cnt = 0;
        }

        if node_type == "DeclRefExpr" {
            state.decl_ref_expr_cnt += 1;
        }

        let node = format!("{}: {}", node_type, node_value,);

        // Print the cursor kind and name
        println!("{}", node);

        state.ast.push(node.to_string());

        // Clean up
        clang_disposeString(kind_spelling);
        clang_disposeString(spelling);

        (node_type, node_value)
    }
}

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
    let _ = state.ast.pop().unwrap();

    for k in 0..state.var_decl {
        /* Pop variable declaration */
        let value = state.cmd_stack.pop().unwrap();

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

    // println!("{:?}", state.variables);
}

fn split(s: String) -> (String, String) {
    let (p1, p2) = s.split_once(":").unwrap();
    let label = p1.trim();
    let value = p2.trim();
    (label.to_string(), value.to_string())
}

fn parse_parm_decl(state: &mut ExecutionState) {
    /* Parse ownership */
    let ownership = state.annotation.contains("MOVE");
    state.annotation.clear();

    /*
     * Pop ParmDecl
     */
    let (l, v) = split(state.ast.pop().unwrap());
    let (mutable, indirection, label) = parse_variable(v);

    let variable = Variable {
        mutable,
        ownership,
        indirection,
    };

    /* Insert variable */
    state.variables.insert(label, variable.clone());

    state.params.get_or_insert_with(Vec::new).push(variable);

    // println!("{:?}", state.variables);
}

fn parse_decl_ref_expr(state: &mut ExecutionState) {
    /* Pop DeclRefExpr */
    let (l, v) = split(state.ast.pop().unwrap());

    state.cmd_stack.push(v);
}

fn parse_unexposed_expr(state: &mut ExecutionState) {
    /* Pop DeclRefExpr */
    let (l, v) = split(state.ast.pop().unwrap());

    /* No side effects */
}

fn parse_call_expr(state: &mut ExecutionState) {
    let _ = split(state.ast.pop().unwrap());

    // todo!(); // Functions with than one arguments??
    // let arg = state.cmd_stack.pop().unwrap();
    // let func = state.cmd_stack.pop().unwrap();

    let mut params = Vec::new();
    for k in 0..state.decl_ref_expr_cnt - 1 {
        params.push(state.cmd_stack.pop().unwrap());
    }
    params.reverse();

    let func = state.cmd_stack.pop().unwrap();

    if func == "ownership_drop" {
        // state.variables.get_mut(&params.first()).unwrap().ownership = false;
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

fn parse_function_decl(state: &mut ExecutionState) {
    let (l, v) = split(state.ast.pop().unwrap());

    let (p1, p2, p3) = split_function_signature(&v).unwrap();

    let mut ret_val: Option<Variable> = None;

    if p1 != "void" {
        let (mutable, indirection, label) = parse_variable(p1);
        ret_val = Some(Variable {
            mutable,
            ownership: true,
            indirection,
        });
    }

    let func = Function {
        params: state.params.take(),
        ret_val,
    };

    state.functions.insert(p3, func);

    // End of function processing - drop all variables
    state.variables.clear();
}

fn parse_binary_operator(state: &mut ExecutionState) {
    /* Pop BinaryOperator */
    let _ = state.ast.pop();

    /* Pop the operands */
    let rhs_label = state.cmd_stack.pop().unwrap();
    let lhs_label = state.cmd_stack.pop().unwrap();

    /* Evaluate correctness */
    // println!("{} = {}", lhs_label, rhs_label);
    let lhs = state.variables.get(&lhs_label);
    let rhs = state.variables.get(&rhs_label);

    if lhs.unwrap().ownership {
        assert!(
            false,
            "ERROR: Transfering ownership to '{}' while variable already holds ownership!",
            lhs_label
        );
    }
}

fn parse_var_decl(state: &mut ExecutionState) {
    /* Pop VarDecl */
    let (_, value) = split(state.ast.pop().unwrap());

    /* Push the raw string onto cmd_stack for future process */
    state.cmd_stack.push(value);

    state.var_decl += 1;
}

fn parse_attribute_annotate(state: &mut ExecutionState) {
    /* Pop annotation */
    let (l, v) = split(state.ast.pop().unwrap());

    state.annotation = v;
}

fn parse_compound_stmt(state: &mut ExecutionState) {
    /* Throw away */
    let _ = split(state.ast.pop().unwrap());
}

fn parse_paren_expr(state: &mut ExecutionState) {
    /* Throw away */
    let _ = split(state.ast.pop().unwrap());
}

extern "C" fn visit_cursor(
    cursor: CXCursor,
    _parent: CXCursor,
    client_data: CXClientData,
) -> CXChildVisitResult {
    let state: &mut ExecutionState = unsafe { &mut *(client_data as *mut ExecutionState) };

    let (node_type, node_value) = pre_process(cursor, state);

    state.indent += 1;
    unsafe {
        clang_visitChildren(cursor, visit_cursor, client_data);
    }
    state.indent -= 1;

    match node_type.as_str() {
        "FunctionDecl" => {
            parse_function_decl(state);
        }
        "CallExpr" => {
            parse_call_expr(state);
        }
        "UnexposedExpr" => {
            parse_unexposed_expr(state);
        }
        "DeclRefExpr" => {
            parse_decl_ref_expr(state);
        }
        "ParmDecl" => {
            parse_parm_decl(state);
        }
        "DeclStmt" => {
            parse_decl_stmt(state);
        }
        "BinaryOperator" => {
            parse_binary_operator(state);
        }
        "VarDecl" => {
            parse_var_decl(state);
        }
        "attribute(annotate)" => {
            parse_attribute_annotate(state);
        }
        "CompoundStmt" => {
            parse_compound_stmt(state);
        }
        "ParenExpr" => {
            parse_paren_expr(state);
        }
        // "ReturnStmt" => {}
        _ => {
            todo!()
        }
    }

    CXChildVisit_Continue
}

struct Component {}

#[derive(Debug, Clone)]
struct Variable {
    mutable: bool,
    ownership: bool,
    indirection: u32,
}

struct Function {
    params: Option<Vec<Variable>>,
    ret_val: Option<Variable>,
}

struct ExecutionState {
    decl_ref_expr_cnt: u32,
    params: Option<Vec<Variable>>,
    variables: HashMap<String, Variable>,
    functions: HashMap<String, Function>,
    indent: u32,
    ast: Vec<String>,
    cmd_stack: Vec<String>,
    annotation: String,
    var_decl: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <source-file>", args[0]);
        return;
    }

    let filename = &args[1];

    unsafe {
        // Create an index
        let index = clang_createIndex(0, 0);

        // Convert filename to CString for libclang
        let c_filename = CString::new(filename.as_str()).expect("CString::new failed");

        // Parse the file into a translation unit
        let translation_unit = clang_parseTranslationUnit(
            index,
            c_filename.as_ptr(),
            ptr::null(),
            0,
            ptr::null_mut(),
            0,
            CXTranslationUnit_None,
        );

        if translation_unit.is_null() {
            eprintln!("Unable to parse translation unit");
            clang_disposeIndex(index);
            return;
        }

        // Get the root cursor of the AST
        let root_cursor = clang_getTranslationUnitCursor(translation_unit);

        let mut state = ExecutionState {
            decl_ref_expr_cnt: 0,
            params: None,
            indent: 0,
            ast: Vec::new(),
            cmd_stack: Vec::new(),
            annotation: String::new(),
            functions: HashMap::new(),
            variables: HashMap::new(),
            var_decl: 0,
        };

        clang_visitChildren(
            root_cursor,
            visit_cursor,
            &mut state as *mut ExecutionState as *mut c_void,
        );

        // Clean up
        clang_disposeTranslationUnit(translation_unit);
        clang_disposeIndex(index);

        println!("{:?}", state.variables);
    }
}

/*
 *  Test #1: a = b
 *  Test #2: *a = *b
 *  Test #3: a = func(a);
 *  Test #4: a = func(a, b);
 *  Test #4: ? = func(a, b);
 *
 */
