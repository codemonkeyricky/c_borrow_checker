
type Label = String;

#[derive(Debug, Clone, Copy)]
pub struct Variable {
    pub mutable: bool,
    pub ownership: bool,
    pub indirection: u32,
}

pub struct TranslationUnit {
    pub sub_unit: Vec<TranslationUnitSet>,
}

pub enum TranslationUnitSet {
    Function(Function),
    /* TODO: and declarations */
}

#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub param: Vec<Variable>,
    pub ret_val: Option<Variable>,
    pub inst: Vec<Inst>,
}

#[derive(Clone)]

pub enum Inst {
    InstSet(u64, Vec<Inst>), // CompoundStmt
    ParamDecl(u64, String, Variable),
    FieldDecl(u64, String, Variable),
    VarDecl(u64, String, Variable),
    Assign(u64, String, ExprDescriptor),
    Eval(u64, ExprDescriptor),
    If(u64, Vec<Inst>), // must be size of 3
    ReturnStmt(u64, String),
}

#[derive(Clone)]
pub enum ExprDescriptor {
    FunctionCall(
        String,              /* func name */
        Vec<ExprDescriptor>, /* func args */
    ),
    LocalVariable(String),
}

pub enum ExprResult {
    DeclaredVariable(String),
    TemporaryVariable(bool),
}