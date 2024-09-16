extern crate clang_sys;

use clang_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::ptr;

fn print_cursor_info(cursor: CXCursor, indentation: u32) {
    unsafe {
        let kind_spelling = clang_getCursorKindSpelling(clang_getCursorKind(cursor));
        let spelling = clang_getCursorSpelling(cursor);

        // Convert C strings to Rust strings
        let kind_cstr = CStr::from_ptr(clang_getCString(kind_spelling));
        let spelling_cstr = CStr::from_ptr(clang_getCString(spelling));

        // Print indentation
        for _ in 0..indentation {
            print!("  ");
        }

        let cursor_type = clang_getCursorType(cursor);
        if clang_isConstQualifiedType(cursor_type) != 0 {
            print!(" [const] ");
        }

        let type_spelling = clang_getTypeSpelling(cursor_type);

        // print!(
        //     " (Type: {})",
        //     CStr::from_ptr(clang_getCString(type_spelling)).to_string_lossy()
        // );

        // Print the cursor kind and name
        println!(
            "{}: {}",
            kind_cstr.to_string_lossy(),
            spelling_cstr.to_string_lossy()
        );

        // Clean up
        clang_disposeString(kind_spelling);
        clang_disposeString(spelling);
    }
}

extern "C" fn visit_cursor(
    cursor: CXCursor,
    _parent: CXCursor,
    client_data: CXClientData,
) -> CXChildVisitResult {
    let indentation = unsafe { *(client_data as *const u32) };
    print_cursor_info(cursor, indentation);

    let new_indentation = indentation + 1;
    unsafe {
        clang_visitChildren(
            cursor,
            visit_cursor,
            &new_indentation as *const u32 as CXClientData,
        );
    }

    CXChildVisit_Continue
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

        // Start visiting AST nodes
        let indentation: u32 = 0;
        clang_visitChildren(
            root_cursor,
            visit_cursor,
            &indentation as *const u32 as *mut c_void,
        );

        // Clean up
        clang_disposeTranslationUnit(translation_unit);
        clang_disposeIndex(index);
    }
}
