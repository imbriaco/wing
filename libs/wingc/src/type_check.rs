mod class_fields_init;
pub(crate) mod jsii_importer;
pub mod lifts;
pub mod symbol_env;

use crate::ast::{self, ClassField, FunctionDefinition, NewExpr, TypeAnnotationKind};
use crate::ast::{
	ArgList, BinaryOperator, Class as AstClass, Expr, ExprKind, FunctionBody, FunctionParameter as AstFunctionParameter,
	Interface as AstInterface, InterpolatedStringPart, Literal, Phase, Reference, Scope, Spanned, Stmt, StmtKind, Symbol,
	TypeAnnotation, UnaryOperator, UserDefinedType,
};
use crate::comp_ctx::{CompilationContext, CompilationPhase};
use crate::diagnostic::{report_diagnostic, Diagnostic, TypeError, WingSpan};
use crate::docs::Docs;
use crate::{
	dbg_panic, debug, WINGSDK_ARRAY, WINGSDK_ASSEMBLY_NAME, WINGSDK_BRINGABLE_MODULES, WINGSDK_DURATION, WINGSDK_JSON,
	WINGSDK_MAP, WINGSDK_MUT_ARRAY, WINGSDK_MUT_JSON, WINGSDK_MUT_MAP, WINGSDK_MUT_SET, WINGSDK_RESOURCE, WINGSDK_SET,
	WINGSDK_STD_MODULE, WINGSDK_STRING,
};
use derivative::Derivative;
use indexmap::{IndexMap, IndexSet};
use itertools::{izip, Itertools};
use jsii_importer::JsiiImporter;

use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::iter::FilterMap;
use std::path::Path;
use symbol_env::{StatementIdx, SymbolEnv};
use wingii::fqn::FQN;
use wingii::type_system::TypeSystem;

use self::class_fields_init::VisitClassInit;
use self::jsii_importer::JsiiImportSpec;
use self::lifts::Lifts;
use self::symbol_env::{LookupResult, SymbolEnvIter, SymbolEnvRef};

pub struct UnsafeRef<T>(*const T);
impl<T> Clone for UnsafeRef<T> {
	fn clone(&self) -> Self {
		Self(self.0)
	}
}

impl<T> Copy for UnsafeRef<T> {}

impl<T> std::ops::Deref for UnsafeRef<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.0 }
	}
}

impl<T> std::ops::DerefMut for UnsafeRef<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *(self.0 as *mut T) }
	}
}

impl<T> Display for UnsafeRef<T>
where
	T: Display,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let t: &T = self;
		write!(f, "{}", t)
	}
}

pub type TypeRef = UnsafeRef<Type>;

#[derive(Debug)]
pub enum SymbolKind {
	Type(TypeRef),
	Variable(VariableInfo),
	Namespace(NamespaceRef),
}

#[derive(Debug, Clone)]
pub enum VariableKind {
	/// a free variable not associated with a specific type
	Free,

	/// an instance member (either of classes or of structs)
	InstanceMember,

	/// a class member (or an enum member)
	StaticMember,

	/// a type (e.g. `std.Json`)
	Type,

	/// a namespace (e.g. `cloud`)
	Namespace,

	/// an error placeholder
	Error,
}

/// Information about a variable in the environment
#[derive(Debug, Clone)]
pub struct VariableInfo {
	/// The name of the variable
	pub name: Symbol,
	/// Type of the variable
	pub type_: TypeRef,
	/// Can the variable be reassigned? (only applies to variables and fields)
	pub reassignable: bool,
	/// The phase in which this variable exists
	pub phase: Phase,
	/// The kind of variable
	pub kind: VariableKind,

	pub docs: Option<Docs>,
}

impl SymbolKind {
	pub fn make_member_variable(
		name: Symbol,
		type_: TypeRef,
		reassignable: bool,
		is_static: bool,
		phase: Phase,
		docs: Option<Docs>,
	) -> Self {
		SymbolKind::Variable(VariableInfo {
			name,
			type_,
			reassignable,
			phase,
			kind: if is_static {
				VariableKind::StaticMember
			} else {
				VariableKind::InstanceMember
			},
			docs,
		})
	}

	pub fn make_free_variable(name: Symbol, type_: TypeRef, reassignable: bool, phase: Phase) -> Self {
		SymbolKind::Variable(VariableInfo {
			name,
			type_,
			reassignable,
			phase,
			kind: VariableKind::Free,
			docs: None,
		})
	}

	pub fn as_variable(&self) -> Option<VariableInfo> {
		match &self {
			SymbolKind::Variable(t) => Some(t.clone()),
			_ => None,
		}
	}

	pub fn as_namespace_ref(&self) -> Option<NamespaceRef> {
		match self {
			SymbolKind::Namespace(ns) => Some(*ns),
			_ => None,
		}
	}

	fn as_namespace(&self) -> Option<&Namespace> {
		match self {
			SymbolKind::Namespace(ns) => Some(ns),
			_ => None,
		}
	}

	fn as_namespace_mut(&mut self) -> Option<&mut Namespace> {
		match self {
			SymbolKind::Namespace(ref mut ns) => Some(ns),
			_ => None,
		}
	}

	pub fn as_type(&self) -> Option<TypeRef> {
		match &self {
			SymbolKind::Type(t) => Some(*t),
			_ => None,
		}
	}
}

#[derive(Debug)]
pub enum Type {
	Anything,
	Number,
	String,
	Duration,
	Boolean,
	Void,
	Json,
	MutJson,
	Nil,
	Unresolved,
	Optional(TypeRef),
	Array(TypeRef),
	MutArray(TypeRef),
	Map(TypeRef),
	MutMap(TypeRef),
	Set(TypeRef),
	MutSet(TypeRef),
	Function(FunctionSignature),
	Class(Class),
	Interface(Interface),
	Struct(Struct),
	Enum(Enum),
}

pub const CLASS_INIT_NAME: &'static str = "init";
pub const CLASS_INFLIGHT_INIT_NAME: &'static str = "$inflight_init";

pub const CLOSURE_CLASS_HANDLE_METHOD: &'static str = "handle";

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Namespace {
	pub name: String,

	#[derivative(Debug = "ignore")]
	pub env: SymbolEnv,

	// Indicate whether all the types in this namespace have been loaded, this is part of our
	// lazy loading mechanism and is used by the lsp's autocomplete in case we need to load
	// the types after initial compilation.
	#[derivative(Debug = "ignore")]
	pub loaded: bool,
}

pub type NamespaceRef = UnsafeRef<Namespace>;

impl Debug for NamespaceRef {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", &**self)
	}
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Class {
	pub name: Symbol,
	pub parent: Option<TypeRef>,  // Must be a Type::Class type
	pub implements: Vec<TypeRef>, // Must be a Type::Interface type
	#[derivative(Debug = "ignore")]
	pub env: SymbolEnv,
	pub fqn: Option<String>,
	pub is_abstract: bool,
	pub type_parameters: Option<Vec<TypeRef>>,
	pub phase: Phase,
	pub docs: Docs,
	pub lifts: Option<Lifts>,

	// Preflight classes are CDK Constructs which means they have a scope and id as their first arguments
	// this is natively supported by wing using the `as` `in` keywords. However theoretically it is possible
	// to have a construct which does not have these arguments, in which case we can't use the `as` `in` keywords
	// and instead the user will need to pass the relevant args to the class's init method.
	pub std_construct_args: bool,
}
impl Class {
	pub(crate) fn set_lifts(&mut self, lifts: Lifts) {
		self.lifts = Some(lifts);
	}

	/// Returns the type of the "handle" method of a closure class or `None` if this is not a closure
	/// class.
	pub fn get_closure_method(&self) -> Option<TypeRef> {
		self
			.methods(true)
			.find(|(name, type_)| name == CLOSURE_CLASS_HANDLE_METHOD && type_.is_inflight_function())
			.map(|(_, t)| t)
	}
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Interface {
	pub name: Symbol,
	pub docs: Docs,
	pub extends: Vec<TypeRef>, // Must be a Type::Interface type
	#[derivative(Debug = "ignore")]
	pub env: SymbolEnv,
}

impl Interface {
	fn is_resource(&self) -> bool {
		// TODO: This should check that the interface extends `IResource` from
		// the SDK, not just any interface with the name `IResource`
		// https://github.com/winglang/wing/issues/2098
		self.name.name == "IResource"
			|| self.extends.iter().any(|i| {
				i.as_interface()
					.expect("Interface extends a type that isn't an interface")
					.name
					.name == "IResource"
			})
	}
}

impl Display for Interface {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let LookupResult::Found(method, _) = self.get_env().lookup_ext(&CLOSURE_CLASS_HANDLE_METHOD.into(), None) {
			let method = method.as_variable().unwrap();
			if method.phase == Phase::Inflight {
				write!(f, "{}", method.type_) // show signature of inflight closure
			} else {
				write!(f, "{}", self.name.name)
			}
		} else {
			write!(f, "{}", self.name.name)
		}
	}
}

type ClassLikeIterator<'a> =
	FilterMap<SymbolEnvIter<'a>, fn(<SymbolEnvIter as Iterator>::Item) -> Option<(String, TypeRef)>>;

pub trait ClassLike {
	fn get_env(&self) -> &SymbolEnv;

	fn methods(&self, with_ancestry: bool) -> ClassLikeIterator<'_> {
		self.get_env().iter(with_ancestry).filter_map(|(s, t, ..)| {
			t.as_variable()
				.unwrap()
				.type_
				.as_function_sig()
				.map(|_| (s.clone(), t.as_variable().unwrap().type_))
		})
	}

	fn fields(&self, with_ancestry: bool) -> ClassLikeIterator<'_> {
		self.get_env().iter(with_ancestry).filter_map(|(s, t, ..)| {
			if t.as_variable().unwrap().type_.as_function_sig().is_none() {
				Some((s, t.as_variable().unwrap().type_))
			} else {
				None
			}
		})
	}

	fn get_method(&self, name: &Symbol) -> Option<VariableInfo> {
		let v = self
			.get_env()
			.lookup_ext(name, None)
			.ok()?
			.0
			.as_variable()
			.expect("class env should only contain variables");
		if v.type_.as_function_sig().is_some() {
			Some(v)
		} else {
			None
		}
	}

	fn get_field(&self, name: &Symbol) -> Option<VariableInfo> {
		self.get_env().lookup_ext(name, None).ok()?.0.as_variable()
	}
}

impl ClassLike for Interface {
	fn get_env(&self) -> &SymbolEnv {
		&self.env
	}
}

impl ClassLike for Class {
	fn get_env(&self) -> &SymbolEnv {
		&self.env
	}
}

impl ClassLike for Struct {
	fn get_env(&self) -> &SymbolEnv {
		&self.env
	}
}

/// Intermediate struct for storing the evaluated types of arguments in a function call or constructor call.
pub struct ArgListTypes {
	pub pos_args: Vec<TypeRef>,
	pub named_args: IndexMap<Symbol, TypeRef>,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Struct {
	pub name: Symbol,
	pub docs: Docs,
	pub extends: Vec<TypeRef>, // Must be a Type::Struct type
	#[derivative(Debug = "ignore")]
	pub env: SymbolEnv,
}

#[derive(Debug)]
pub struct Enum {
	pub name: Symbol,
	pub docs: Docs,
	pub values: IndexSet<Symbol>,
}

#[derive(Debug)]
pub struct EnumInstance {
	pub enum_name: TypeRef,
	pub enum_value: Symbol,
}

trait Subtype {
	/// Returns true if `self` is a subtype of `other`.
	///
	/// For example, `str` is a subtype of `str`, `str` is a subtype of
	/// `anything`, `str` is a subtype of `Json`, `str` is not a subtype of
	/// `num`, and `str` is not a subtype of `void`.
	///
	/// Subtype is a partial order, so if a.is_subtype_of(b) is false, it does
	/// not imply that b.is_subtype_of(a) is true. It is also reflexive, so
	/// a.is_subtype_of(a) is always true.
	///
	/// TODO: change return type to allow additional subtyping information to be
	/// returned, for better error messages when one type isn't the subtype of another.
	fn is_subtype_of(&self, other: &Self) -> bool;

	fn is_same_type_as(&self, other: &Self) -> bool {
		self.is_subtype_of(other) && other.is_subtype_of(self)
	}

	fn is_strict_subtype_of(&self, other: &Self) -> bool {
		self.is_subtype_of(other) && !other.is_subtype_of(self)
	}
}

impl Subtype for Phase {
	fn is_subtype_of(&self, other: &Self) -> bool {
		// We model phase subtyping as if the independent phase is an
		// intersection type of preflight and inflight. This means that
		// independent = preflight & inflight.
		//
		// This means the following pseudocode is valid:
		// > let x: preflight fn = <phase-independent function>;
		// (a phase-independent function is a subtype of a preflight function)
		//
		// But the following pseudocode is not valid:
		// > let x: independent fn = <preflight function>;
		// (a preflight function is not a subtype of an inflight function)
		if self == &Phase::Independent {
			true
		} else {
			self == other
		}
	}
}

impl Subtype for Type {
	fn is_subtype_of(&self, other: &Self) -> bool {
		// If references are the same this is the same type, if not then compare content
		if std::ptr::eq(self, other) {
			return true;
		}
		match (self, other) {
			(Self::Anything, _) | (_, Self::Anything) => {
				// TODO: Hack to make anything's compatible with all other types, specifically useful for handling core.Inflight handlers
				true
			}
			(Self::Function(l0), Self::Interface(r0)) => {
				// TODO: Hack to make functions compatible with interfaces
				// Remove this after https://github.com/winglang/wing/issues/1448

				// First check that the function is in the inflight phase
				if l0.phase != Phase::Inflight {
					return false;
				}

				// Next, compare the function to a method on the interface named "handle" if it exists
				if let Some((method, _)) = r0.get_env().lookup_ext(&CLOSURE_CLASS_HANDLE_METHOD.into(), None).ok() {
					let method = method.as_variable().unwrap();
					if method.phase != Phase::Inflight {
						return false;
					}

					return self.is_subtype_of(&*method.type_);
				}

				false
			}
			(Self::Function(l0), Self::Function(r0)) => {
				if !l0.phase.is_subtype_of(&r0.phase) {
					return false;
				}

				// If the return types are not subtypes of each other, then this is not a subtype
				// exception: if function type we are assigning to returns void, then any return type is ok
				if !l0.return_type.is_subtype_of(&r0.return_type) && !(r0.return_type.is_void()) {
					return false;
				}

				// In this section, we check if the parameter types are not subtypes of each other, then this is not a subtype.

				// Check that this function has at most as many required parameters as the other function requires
				// if it doesn't, we know it's not a subtype
				if l0.min_parameters() > r0.min_parameters() {
					return false;
				}

				let lparams = l0.parameters.iter();
				let rparams = r0.parameters.iter();

				for (l, r) in lparams.zip(rparams) {
					// parameter types are contravariant, which means even if Cat is a subtype of Animal,
					// (Cat) => void is not a subtype of (Animal) => void
					// but (Animal) => void is a subtype of (Cat) => void
					// see https://en.wikipedia.org/wiki/Covariance_and_contravariance_(computer_science)
					if !r.typeref.is_subtype_of(&l.typeref) {
						return false;
					}
				}
				true
			}
			(Self::Class(l0), Self::Class(_)) => {
				// If we extend from `other` then I'm a subtype of it (inheritance)
				if let Some(parent) = l0.parent.as_ref() {
					let parent_type: &Type = parent;
					return parent_type.is_subtype_of(other);
				}
				false
			}
			(Self::Interface(l0), Self::Interface(_)) => {
				// If we extend from `other` then I'm a subtype of it (inheritance)
				l0.extends.iter().any(|parent| {
					let parent_type: &Type = parent;
					parent_type.is_subtype_of(other)
				})
			}
			(Self::Class(class), Self::Interface(iface)) => {
				// If a resource implements the interface then it's a subtype of it (nominal typing)
				let implements_iface = class.implements.iter().any(|parent| {
					let parent_type: &Type = parent;
					parent_type.is_subtype_of(other)
				});

				let base_class_implements_iface = if let Some(base_class) = &class.parent {
					let base_class_type: &Type = base_class;
					base_class_type.is_subtype_of(other)
				} else {
					false
				};

				if implements_iface || base_class_implements_iface {
					return true;
				}

				// To support flexible inflight closures, we say that any class with an inflight method
				// named "handle" is a subtype of any single-method interface with a matching "handle"
				// method type (aka "closure classes").

				// First, check if there is exactly one inflight method in the interface
				let mut inflight_methods = iface
					.methods(true)
					.filter(|(_name, type_)| type_.is_inflight_function());
				let handler_method = inflight_methods.next();
				if handler_method.is_none() || inflight_methods.next().is_some() {
					return false;
				}

				// Next, check that the method's name is "handle"
				let (handler_method_name, handler_method_type) = handler_method.unwrap();
				if handler_method_name != CLOSURE_CLASS_HANDLE_METHOD {
					return false;
				}

				// Then get the type of the resource's "handle" method if it has one
				let res_handle_type = if let Some(method) = class.get_method(&CLOSURE_CLASS_HANDLE_METHOD.into()) {
					if method.type_.is_inflight_function() {
						method.type_
					} else {
						return false;
					}
				} else {
					return false;
				};

				// Finally check if they're subtypes
				res_handle_type.is_subtype_of(&handler_method_type)
			}
			(Self::Class(res), Self::Function(_)) => {
				// To support flexible inflight closures, we say that any
				// preflight class with an inflight method named "handle" is a subtype of
				// any matching inflight type.

				// Get the type of the resource's "handle" method if it has one
				let res_handle_type = if let Some(method) = res.get_method(&CLOSURE_CLASS_HANDLE_METHOD.into()) {
					if method.type_.is_inflight_function() {
						method.type_
					} else {
						return false;
					}
				} else {
					return false;
				};

				// Finally check if they're subtypes
				(*res_handle_type).is_subtype_of(other)
			}
			(_, Self::Interface(_)) => {
				// TODO - for now only resources can implement interfaces
				// https://github.com/winglang/wing/issues/2111
				false
			}
			(Self::Struct(l0), Self::Struct(_)) => {
				// If we extend from `other` then I'm a subtype of it (inheritance)
				for parent in l0.extends.iter() {
					let parent_type: &Type = parent;
					if parent_type.is_subtype_of(other) {
						return true;
					}
				}
				false
			}
			(Self::Array(l0), Self::Array(r0)) => {
				// An Array type is a subtype of another Array type if the value type is a subtype of the other value type
				let l: &Type = l0;
				let r: &Type = r0;
				l.is_subtype_of(r)
			}
			(Self::MutArray(l0), Self::MutArray(r0)) => {
				// An Array type is a subtype of another Array type if the value type is a subtype of the other value type
				let l: &Type = l0;
				let r: &Type = r0;
				l.is_subtype_of(r)
			}
			(Self::Map(l0), Self::Map(r0)) => {
				// A Map type is a subtype of another Map type if the value type is a subtype of the other value type
				let l: &Type = l0;
				let r: &Type = r0;
				l.is_subtype_of(r)
			}
			(Self::MutMap(l0), Self::MutMap(r0)) => {
				// A Map type is a subtype of another Map type if the value type is a subtype of the other value type
				let l: &Type = l0;
				let r: &Type = r0;
				l.is_subtype_of(r)
			}
			(Self::Set(l0), Self::Set(r0)) => {
				// A Set type is a subtype of another Set type if the value type is a subtype of the other value type
				let l: &Type = l0;
				let r: &Type = r0;
				l.is_subtype_of(r)
			}
			(Self::MutSet(l0), Self::MutSet(r0)) => {
				// A Set type is a subtype of another Set type if the value type is a subtype of the other value type
				let l: &Type = l0;
				let r: &Type = r0;
				l.is_subtype_of(r)
			}
			(Self::Enum(e0), Self::Enum(e1)) => {
				// An enum type is a subtype of another Enum type only if they are the exact same
				e0.name == e1.name
			}
			(Self::Optional(l0), Self::Optional(r0)) => {
				// An Optional type is a subtype of another Optional type if the value type is a subtype of the other value type
				let l: &Type = l0;
				let r: &Type = r0;
				l.is_subtype_of(r)
			}
			(Self::Nil, Self::Optional(_)) => {
				// Nil is a subtype of Optional<T> for any T
				true
			}
			(_, Self::Optional(r0)) => {
				// A non-Optional type is a subtype of an Optional type if the non-optional's type is a subtype of the value type
				// e.g. `String` is a subtype of `Optional<String>`
				let r: &Type = r0;
				self.is_subtype_of(r)
			}
			(Self::Number, Self::Number) => true,
			(Self::String, Self::String) => true,
			(Self::Boolean, Self::Boolean) => true,
			(Self::Duration, Self::Duration) => true,
			(Self::Void, Self::Void) => true,
			_ => false,
		}
	}
}

#[derive(Clone, Debug)]
pub struct FunctionParameter {
	pub name: String,
	pub typeref: TypeRef,
	pub docs: Docs,
}

#[derive(Clone, Debug)]
pub struct FunctionSignature {
	/// The type of "this" inside the function, if any. This should be None for
	/// static or anonymous functions.
	pub this_type: Option<TypeRef>,
	pub parameters: Vec<FunctionParameter>,
	pub return_type: TypeRef,
	pub phase: Phase,
	/// During jsify, calls to this function will be replaced with this string
	/// In JSII imports, this is denoted by the `@macro` attribute
	/// This string may contain special tokens:
	/// - `$self$`: The expression on which this function was called
	/// - `$args$`: the arguments passed to this function call
	/// - `$args_text$`: the original source text of the arguments passed to this function call, escaped
	pub js_override: Option<String>,
	pub docs: Docs,
}

impl FunctionSignature {
	/// Returns the minimum number of parameters that need to be passed to this function.
	fn min_parameters(&self) -> usize {
		// Count number of optional parameters from the end of the constructor's params
		// Allow arg_list to be missing up to that number of option (or any) values to try and make the number of arguments match
		let num_optionals = self
			.parameters
			.iter()
			.rev()
			// TODO - as a hack we treat `anything` arguments like optionals so that () => {} can be a subtype of (any) => {}
			.take_while(|arg| arg.typeref.is_option() || arg.typeref.is_struct() || arg.typeref.is_anything())
			.count();

		self.parameters.len() - num_optionals
	}
}

impl PartialEq for FunctionSignature {
	fn eq(&self, other: &Self) -> bool {
		self
			.parameters
			.iter()
			.zip(other.parameters.iter())
			.all(|(x, y)| x.typeref.is_same_type_as(&y.typeref))
			&& self.return_type.is_same_type_as(&other.return_type)
			&& self.phase == other.phase
	}
}

impl Display for SymbolKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SymbolKind::Type(_) => write!(f, "type"),
			SymbolKind::Variable(_) => write!(f, "variable"),
			SymbolKind::Namespace(_) => write!(f, "namespace"),
		}
	}
}

impl Display for Class {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(closure) = self.get_closure_method() {
			std::fmt::Display::fmt(&closure, f)
		} else {
			write!(f, "{}", self.name.name)
		}
	}
}

impl Display for Type {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Type::Anything => write!(f, "any"),
			Type::Number => write!(f, "num"),
			Type::String => write!(f, "str"),
			Type::Duration => write!(f, "duration"),
			Type::Boolean => write!(f, "bool"),
			Type::Void => write!(f, "void"),
			Type::Json => write!(f, "Json"),
			Type::MutJson => write!(f, "MutJson"),
			Type::Nil => write!(f, "nil"),
			Type::Unresolved => write!(f, "unresolved"),
			Type::Optional(v) => write!(f, "{}?", v),
			Type::Function(sig) => write!(f, "{}", sig),
			Type::Class(class) => write!(f, "{}", class),

			Type::Interface(iface) => write!(f, "{}", iface),
			Type::Struct(s) => write!(f, "{}", s.name.name),
			Type::Array(v) => write!(f, "Array<{}>", v),
			Type::MutArray(v) => write!(f, "MutArray<{}>", v),
			Type::Map(v) => write!(f, "Map<{}>", v),
			Type::MutMap(v) => write!(f, "MutMap<{}>", v),
			Type::Set(v) => write!(f, "Set<{}>", v),
			Type::MutSet(v) => write!(f, "MutSet<{}>", v),
			Type::Enum(s) => write!(f, "{}", s.name.name),
		}
	}
}

impl Display for FunctionSignature {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let phase_str = match self.phase {
			Phase::Inflight => "inflight ",
			Phase::Preflight => "preflight ",
			Phase::Independent => "",
		};
		let params_str = self
			.parameters
			.iter()
			.map(|a| {
				if a.name.is_empty() {
					format!("{}", a.typeref)
				} else {
					format!("{}: {}", a.name, a.typeref)
				}
			})
			.collect::<Vec<String>>()
			.join(", ");

		let ret_type_str = self.return_type.to_string();
		write!(f, "{phase_str}({params_str}): {ret_type_str}")
	}
}

// TODO Allows for use in async runtime
// TODO either avoid shared memory or use Arc<Mutex<...>> instead
unsafe impl Send for TypeRef {}

impl TypeRef {
	pub fn as_preflight_class(&self) -> Option<&Class> {
		if let Type::Class(ref class) = **self {
			if class.phase == Phase::Preflight {
				return Some(class);
			}
		}

		None
	}

	pub fn as_class_mut(&mut self) -> Option<&mut Class> {
		match **self {
			Type::Class(ref mut class) => Some(class),
			_ => None,
		}
	}

	pub fn as_class(&self) -> Option<&Class> {
		if let Type::Class(ref class) = **self {
			return Some(class);
		}

		None
	}

	pub fn as_struct(&self) -> Option<&Struct> {
		if let Type::Struct(ref s) = **self {
			Some(s)
		} else {
			None
		}
	}

	pub fn as_interface(&self) -> Option<&Interface> {
		if let Type::Interface(ref iface) = **self {
			Some(iface)
		} else {
			None
		}
	}
	fn as_mut_interface(&mut self) -> Option<&mut Interface> {
		if let Type::Interface(ref mut iface) = **self {
			Some(iface)
		} else {
			None
		}
	}

	pub fn maybe_unwrap_option(&self) -> &Self {
		if let Type::Optional(ref t) = **self {
			t
		} else {
			self
		}
	}

	pub fn as_function_sig(&self) -> Option<&FunctionSignature> {
		if let Type::Function(ref sig) = **self {
			Some(sig)
		} else {
			None
		}
	}

	pub fn as_mut_function_sig(&mut self) -> Option<&mut FunctionSignature> {
		if let Type::Function(ref mut sig) = **self {
			Some(sig)
		} else {
			None
		}
	}

	pub fn is_anything(&self) -> bool {
		matches!(**self, Type::Anything)
	}

	pub fn is_unresolved(&self) -> bool {
		matches!(**self, Type::Unresolved)
	}

	pub fn is_json(&self) -> bool {
		if let Type::Json | Type::MutJson = **self {
			return true;
		} else {
			false
		}
	}

	pub fn is_preflight_class(&self) -> bool {
		if let Type::Class(ref class) = **self {
			return class.phase == Phase::Preflight;
		}

		return false;
	}

	/// Returns whether type represents a closure (either a function or a closure class).
	pub fn is_closure(&self) -> bool {
		if self.as_function_sig().is_some() {
			return true;
		}

		if let Some(ref class) = self.as_preflight_class() {
			return class.get_closure_method().is_some();
		}
		false
	}

	pub fn is_string(&self) -> bool {
		matches!(**self, Type::String)
	}

	pub fn is_struct(&self) -> bool {
		matches!(**self, Type::Struct(_))
	}

	pub fn is_void(&self) -> bool {
		matches!(**self, Type::Void)
	}

	pub fn is_option(&self) -> bool {
		matches!(**self, Type::Optional(_))
	}

	pub fn is_immutable_collection(&self) -> bool {
		matches!(**self, Type::Array(_) | Type::Map(_) | Type::Set(_))
	}

	pub fn is_inflight_function(&self) -> bool {
		if let Type::Function(ref sig) = **self {
			sig.phase == Phase::Inflight
		} else {
			false
		}
	}

	/// If this is a function and its last argument is a struct, return that struct.
	pub fn get_function_struct_arg(&self) -> Option<&Struct> {
		if let Some(func) = self.maybe_unwrap_option().as_function_sig() {
			if let Some(arg) = func.parameters.last() {
				return arg.typeref.maybe_unwrap_option().as_struct();
			}
		}

		None
	}

	/// Returns the item type of a collection type, or None if the type is not a collection.
	pub fn collection_item_type(&self) -> Option<TypeRef> {
		match **self {
			Type::Array(t) => Some(t),
			Type::MutArray(t) => Some(t),
			Type::Map(t) => Some(t),
			Type::MutMap(t) => Some(t),
			Type::Set(t) => Some(t),
			Type::MutSet(t) => Some(t),
			_ => None,
		}
	}

	pub fn is_mutable_collection(&self) -> bool {
		matches!(**self, Type::MutArray(_) | Type::MutMap(_) | Type::MutSet(_))
	}

	pub fn is_iterable(&self) -> bool {
		matches!(
			**self,
			Type::Array(_) | Type::Set(_) | Type::MutArray(_) | Type::MutSet(_)
		)
	}

	pub fn is_capturable(&self) -> bool {
		match &**self {
			Type::Interface(iface) => iface.is_resource(),
			Type::Enum(_) => true,
			Type::Number => true,
			Type::String => true,
			Type::Duration => true,
			Type::Boolean => true,
			Type::Json => true,
			Type::Nil => true,
			Type::Array(v) => v.is_capturable(),
			Type::Map(v) => v.is_capturable(),
			Type::Set(v) => v.is_capturable(),
			Type::Struct(_) => true,
			Type::Optional(v) => v.is_capturable(),
			Type::Anything => false,
			Type::Unresolved => false,
			Type::Void => false,
			Type::MutJson => true,
			Type::MutArray(v) => v.is_capturable(),
			Type::MutMap(v) => v.is_capturable(),
			Type::MutSet(v) => v.is_capturable(),
			Type::Function(sig) => sig.phase == Phase::Inflight,

			// only preflight classes can be captured
			Type::Class(c) => c.phase == Phase::Preflight,
		}
	}

	// returns true if mutable type or if immutable container type contains a mutable type
	pub fn is_mutable(&self) -> bool {
		match &**self {
			Type::MutArray(_) => true,
			Type::MutMap(_) => true,
			Type::MutSet(_) => true,
			Type::MutJson => true,
			Type::Array(v) => v.is_mutable(),
			Type::Map(v) => v.is_mutable(),
			Type::Set(v) => v.is_mutable(),
			Type::Optional(v) => v.is_mutable(),
			_ => false,
		}
	}

	pub fn is_nil(&self) -> bool {
		match &**self {
			Type::Nil => true,
			Type::Array(t) => t.is_nil(),
			Type::MutArray(t) => t.is_nil(),
			Type::Map(t) => t.is_nil(),
			Type::MutMap(t) => t.is_nil(),
			Type::Set(t) => t.is_nil(),
			Type::MutSet(t) => t.is_nil(),
			_ => false,
		}
	}

	pub fn is_json_legal_value(&self) -> bool {
		match **self {
			Type::Number => true,
			Type::String => true,
			Type::Boolean => true,
			Type::Json | Type::MutJson => true,
			Type::Array(v) => v.is_json_legal_value(),
			Type::Optional(v) => v.is_json_legal_value(),
			_ => false,
		}
	}
}

impl Subtype for TypeRef {
	fn is_subtype_of(&self, other: &Self) -> bool {
		// Types are equal if they point to the same type definition
		if self.0 == other.0 {
			true
		} else {
			// If the self and other aren't the the same, we need to use the specific types equality function
			let t1: &Type = self;
			let t2: &Type = other;
			t1.is_subtype_of(t2)
		}
	}
}

impl Debug for TypeRef {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", &**self)
	}
}

struct ResolvedExpression {
	type_: TypeRef,
	phase: Phase,
}
pub struct Types {
	// TODO: Remove the box and change TypeRef and NamespaceRef to just be indices into the types array and namespaces array respectively
	// Note: we need the box so reallocations of the vec while growing won't change the addresses of the types since they are referenced from the TypeRef struct
	types: Vec<Box<Type>>,
	namespaces: Vec<Box<Namespace>>,
	pub libraries: SymbolEnv,
	numeric_idx: usize,
	string_idx: usize,
	bool_idx: usize,
	duration_idx: usize,
	anything_idx: usize,
	void_idx: usize,
	json_idx: usize,
	mut_json_idx: usize,
	nil_idx: usize,
	err_idx: usize,

	type_for_expr: Vec<Option<ResolvedExpression>>,

	resource_base_type: Option<TypeRef>,
}

impl Types {
	pub fn new() -> Self {
		let mut types = vec![];
		types.push(Box::new(Type::Number));
		let numeric_idx = types.len() - 1;
		types.push(Box::new(Type::String));
		let string_idx = types.len() - 1;
		types.push(Box::new(Type::Boolean));
		let bool_idx = types.len() - 1;
		types.push(Box::new(Type::Duration));
		let duration_idx = types.len() - 1;
		types.push(Box::new(Type::Anything));
		let anything_idx = types.len() - 1;
		types.push(Box::new(Type::Void));
		let void_idx = types.len() - 1;
		types.push(Box::new(Type::Json));
		let json_idx = types.len() - 1;
		types.push(Box::new(Type::MutJson));
		let mut_json_idx = types.len() - 1;
		types.push(Box::new(Type::Nil));
		let nil_idx = types.len() - 1;
		types.push(Box::new(Type::Unresolved));
		let err_idx = types.len() - 1;

		// TODO: this is hack to create the top-level mapping from lib names to symbols
		// We construct a void ref by hand since we can't call self.void() while constructing the Types struct
		let void_ref = UnsafeRef::<Type>(&*types[void_idx] as *const Type);
		let libraries = SymbolEnv::new(None, void_ref, false, false, Phase::Preflight, 0);

		Self {
			types,
			namespaces: Vec::new(),
			libraries,
			numeric_idx,
			string_idx,
			bool_idx,
			duration_idx,
			anything_idx,
			void_idx,
			json_idx,
			mut_json_idx,
			nil_idx,
			err_idx,
			type_for_expr: Vec::new(),
			resource_base_type: None,
		}
	}

	pub fn number(&self) -> TypeRef {
		self.get_typeref(self.numeric_idx)
	}

	pub fn string(&self) -> TypeRef {
		self.get_typeref(self.string_idx)
	}

	pub fn nil(&self) -> TypeRef {
		self.get_typeref(self.nil_idx)
	}

	pub fn bool(&self) -> TypeRef {
		self.get_typeref(self.bool_idx)
	}

	pub fn duration(&self) -> TypeRef {
		self.get_typeref(self.duration_idx)
	}

	pub fn anything(&self) -> TypeRef {
		self.get_typeref(self.anything_idx)
	}

	pub fn error(&self) -> TypeRef {
		self.get_typeref(self.err_idx)
	}

	pub fn void(&self) -> TypeRef {
		self.get_typeref(self.void_idx)
	}

	pub fn add_type(&mut self, t: Type) -> TypeRef {
		self.types.push(Box::new(t));
		self.get_typeref(self.types.len() - 1)
	}

	/// Get the optional version of a given type.
	///
	/// If the type is already optional, return it as-is.
	pub fn make_option(&mut self, t: TypeRef) -> TypeRef {
		if t.is_option() {
			t
		} else {
			self.add_type(Type::Optional(t))
		}
	}

	fn get_typeref(&self, idx: usize) -> TypeRef {
		let t = &self.types[idx];
		UnsafeRef::<Type>(&**t as *const Type)
	}

	pub fn json(&self) -> TypeRef {
		self.get_typeref(self.json_idx)
	}

	pub fn mut_json(&self) -> TypeRef {
		self.get_typeref(self.mut_json_idx)
	}

	pub fn stringables(&self) -> Vec<TypeRef> {
		// TODO: This should be more complex and return all types that have some stringification facility
		// see: https://github.com/winglang/wing/issues/741
		vec![
			self.string(),
			self.number(),
			self.json(),
			self.mut_json(),
			self.anything(),
		]
	}

	pub fn add_namespace(&mut self, n: Namespace) -> NamespaceRef {
		self.namespaces.push(Box::new(n));
		self.get_namespaceref(self.namespaces.len() - 1)
	}

	fn get_namespaceref(&self, idx: usize) -> NamespaceRef {
		let t = &self.namespaces[idx];
		UnsafeRef::<Namespace>(&**t as *const Namespace)
	}

	fn resource_base_type(&mut self) -> TypeRef {
		// cache the resource base type ref
		if self.resource_base_type.is_none() {
			let resource_fqn = format!("{}.{}", WINGSDK_ASSEMBLY_NAME, WINGSDK_RESOURCE);
			self.resource_base_type = Some(
				self
					.libraries
					.lookup_nested_str(&resource_fqn, None)
					.unwrap()
					.0
					.as_type()
					.unwrap(),
			);
		}

		self.resource_base_type.unwrap()
	}

	/// Stores the type and phase of a given expression node.
	pub fn assign_type_to_expr(&mut self, expr: &Expr, type_: TypeRef, phase: Phase) {
		let expr_idx = expr.id;
		if self.type_for_expr.len() <= expr_idx {
			self.type_for_expr.resize_with(expr_idx + 1, || None);
		}
		self.type_for_expr[expr_idx] = Some(ResolvedExpression { type_, phase });
	}

	/// Obtain the type of a given expression node. Will panic if the expression has not been type checked yet.
	pub fn get_expr_type(&self, expr: &Expr) -> TypeRef {
		self
			.try_get_expr_type(expr)
			.expect("All expressions should have a type")
	}

	/// Obtain the type of a given expression node. Returns None if the expression has not been type checked yet. If
	/// this is called after type checking, it should always return Some.
	pub fn try_get_expr_type(&self, expr: &Expr) -> Option<TypeRef> {
		self
			.type_for_expr
			.get(expr.id)
			.and_then(|t| t.as_ref().map(|t| t.type_))
	}

	pub fn get_expr_phase(&self, expr: &Expr) -> Option<Phase> {
		self
			.type_for_expr
			.get(expr.id)
			.and_then(|t| t.as_ref().map(|t| t.phase))
	}

	/// Given an unqualified type name of a builtin type, return the full type info.
	///
	/// This is needed because our builtin types have no API.
	/// So we have to get the API from the std lib
	/// but the std lib sometimes doesn't have the same names as the builtin types
	/// https://github.com/winglang/wing/issues/1780
	///
	/// Note: This Doesn't handle generics (i.e. this keeps the `T1`)
	pub fn get_std_class(&self, type_: &str) -> Option<(&SymbolKind, symbol_env::SymbolLookupInfo)> {
		let type_name = fully_qualify_std_type(type_);

		let fqn = format!("{WINGSDK_ASSEMBLY_NAME}.{type_name}");

		self.libraries.lookup_nested_str(fqn.as_str(), None).ok()
	}
}

pub struct TypeChecker<'a> {
	types: &'a mut Types,

	/// Scratchpad for storing inner scopes so we can do breadth first traversal of the AST tree during type checking
	///
	/// TODO: this is a list of unsafe pointers to the statement's inner scopes. We use
	/// unsafe because we can't return a mutable reference to the inner scopes since this method
	/// already uses references to the statement that contains the scopes. Using unsafe here just
	/// makes it a lot simpler. Ideally we should avoid returning anything here and have some way
	/// to iterate over the inner scopes given the outer scope. For this we need to model our AST
	/// so all nodes implement some basic "tree" interface. For now this is good enough.
	inner_scopes: Vec<*const Scope>,

	/// The path to the source file being type checked.
	source_path: &'a Path,

	/// JSII Manifest descriptions to be imported.
	/// May be reused between compilations
	jsii_imports: &'a mut Vec<JsiiImportSpec>,

	/// The JSII type system
	jsii_types: &'a mut TypeSystem,

	// Nesting level within JSON literals, a value larger than 0 means we're currently in a JSON literal
	in_json: u64,

	is_in_mut_json: bool,

	/// Index of the current statement being type checked within the current scope
	statement_idx: usize,
}

impl<'a> TypeChecker<'a> {
	pub fn new(
		types: &'a mut Types,
		source_path: &'a Path,
		jsii_types: &'a mut TypeSystem,
		jsii_imports: &'a mut Vec<JsiiImportSpec>,
	) -> Self {
		Self {
			types,
			inner_scopes: vec![],
			jsii_types,
			source_path,
			jsii_imports,
			in_json: 0,
			is_in_mut_json: false,
			statement_idx: 0,
		}
	}

	pub fn add_globals(&mut self, scope: &Scope) {
		self.add_module_to_env(
			scope.env.borrow_mut().as_mut().unwrap(),
			WINGSDK_ASSEMBLY_NAME.to_string(),
			vec![WINGSDK_STD_MODULE.to_string()],
			&Symbol::global(WINGSDK_STD_MODULE),
			None,
		);
	}

	fn spanned_error_with_var<S: Into<String>>(&self, spanned: &impl Spanned, message: S) -> (VariableInfo, Phase) {
		report_diagnostic(Diagnostic {
			message: message.into(),
			span: Some(spanned.span()),
		});

		(self.make_error_variable_info(), Phase::Independent)
	}

	fn spanned_error<S: Into<String>>(&self, spanned: &impl Spanned, message: S) {
		report_diagnostic(Diagnostic {
			message: message.into(),
			span: Some(spanned.span()),
		});
	}

	fn unspanned_error<S: Into<String>>(&self, message: S) {
		report_diagnostic(Diagnostic {
			message: message.into(),
			span: None,
		});
	}

	fn type_error(&self, type_error: TypeError) -> TypeRef {
		let TypeError { message, span } = type_error;
		report_diagnostic(Diagnostic {
			message,
			span: Some(span),
		});

		self.types.error()
	}

	fn make_error_variable_info(&self) -> VariableInfo {
		VariableInfo {
			name: "<error>".into(),
			type_: self.types.error(),
			reassignable: false,
			phase: Phase::Independent,
			kind: VariableKind::Error,
			docs: None,
		}
	}

	// Validates types in the expression make sense and returns the expression's inferred type
	fn type_check_exp(&mut self, exp: &Expr, env: &SymbolEnv) -> (TypeRef, Phase) {
		CompilationContext::set(CompilationPhase::TypeChecking, &exp.span);
		let (t, phase) = self.type_check_exp_helper(&exp, env);
		self.types.assign_type_to_expr(exp, t, phase);
		(t, phase)
	}

	/// Helper function for type_check_exp. This is needed because we want to be able to `return`
	/// and break early, while still setting the evaluated type on the expression.
	///
	/// Do not use this function directly, use `type_check_exp` instead.
	fn type_check_exp_helper(&mut self, exp: &Expr, env: &SymbolEnv) -> (TypeRef, Phase) {
		match &exp.kind {
			ExprKind::Literal(lit) => match lit {
				Literal::String(_) => (self.types.string(), Phase::Independent),
				Literal::Nil => (self.types.nil(), Phase::Independent),
				Literal::InterpolatedString(s) => {
					s.parts.iter().for_each(|part| {
						if let InterpolatedStringPart::Expr(interpolated_expr) = part {
							let (exp_type, _) = self.type_check_exp(interpolated_expr, env);
							self.validate_type_in(exp_type, &self.types.stringables(), interpolated_expr);
						}
					});
					(self.types.string(), Phase::Independent)
				}
				Literal::Number(_) => (self.types.number(), Phase::Independent),
				Literal::Boolean(_) => (self.types.bool(), Phase::Independent),
			},
			ExprKind::Binary { op, left, right } => {
				let (ltype, ltype_phase) = self.type_check_exp(left, env);
				let (rtype, _) = self.type_check_exp(right, env);

				match op {
					BinaryOperator::LogicalAnd | BinaryOperator::LogicalOr => {
						self.validate_type(ltype, self.types.bool(), left);
						self.validate_type(rtype, self.types.bool(), right);
						(self.types.bool(), Phase::Independent)
					}
					BinaryOperator::AddOrConcat => {
						if ltype.is_subtype_of(&self.types.number()) && rtype.is_subtype_of(&self.types.number()) {
							(self.types.number(), Phase::Independent)
						} else if ltype.is_subtype_of(&self.types.string()) && rtype.is_subtype_of(&self.types.string()) {
							(self.types.string(), Phase::Independent)
						} else {
							// If any of the types are unresolved (error) then don't report this assuming the error has already been reported
							if !ltype.is_unresolved() && !rtype.is_unresolved() {
								self.spanned_error(
									exp,
									format!(
										"Binary operator '+' cannot be applied to operands of type '{}' and '{}'; only ({}, {}) and ({}, {}) are supported",
										ltype, rtype, self.types.number(), self.types.number(), self.types.string(), self.types.string(),
									),
								);
							}
							self.resolved_error()
						}
					}
					BinaryOperator::Sub
					| BinaryOperator::Mul
					| BinaryOperator::Div
					| BinaryOperator::FloorDiv
					| BinaryOperator::Mod
					| BinaryOperator::Power => {
						self.validate_type(ltype, self.types.number(), left);
						self.validate_type(rtype, self.types.number(), right);
						(self.types.number(), Phase::Independent)
					}
					BinaryOperator::Equal | BinaryOperator::NotEqual => {
						self.validate_type(rtype, ltype, exp);
						(self.types.bool(), Phase::Independent)
					}
					BinaryOperator::Less
					| BinaryOperator::LessOrEqual
					| BinaryOperator::Greater
					| BinaryOperator::GreaterOrEqual => {
						self.validate_type(ltype, self.types.number(), left);
						self.validate_type(rtype, self.types.number(), right);
						(self.types.bool(), Phase::Independent)
					}
					BinaryOperator::UnwrapOr => {
						// Left argument must be an optional type
						if !ltype.is_option() {
							self.spanned_error(left, format!("Expected optional type, found \"{}\"", ltype));
							(ltype, ltype_phase)
						} else {
							// Right argument must be a subtype of the inner type of the left argument
							let inner_type = *ltype.maybe_unwrap_option();
							self.validate_type(rtype, inner_type, right);
							(inner_type, ltype_phase)
						}
					}
				}
			}
			ExprKind::Unary { op, exp: unary_exp } => {
				let (type_, phase) = self.type_check_exp(unary_exp, env);

				match op {
					UnaryOperator::Not => (self.validate_type(type_, self.types.bool(), unary_exp), phase),
					UnaryOperator::Minus => (self.validate_type(type_, self.types.number(), unary_exp), phase),
					UnaryOperator::OptionalTest => {
						if !type_.is_option() {
							self.spanned_error(unary_exp, format!("Expected optional type, found \"{}\"", type_));
						}
						(self.types.bool(), phase)
					}
				}
			}
			ExprKind::Range {
				start,
				inclusive: _,
				end,
			} => {
				let (stype, stype_phase) = self.type_check_exp(start, env);
				let (etype, _) = self.type_check_exp(end, env);

				self.validate_type(stype, self.types.number(), start);
				self.validate_type(etype, self.types.number(), end);
				(self.types.add_type(Type::Array(stype)), stype_phase)
			}
			ExprKind::Reference(_ref) => {
				let (vi, phase) = self.resolve_reference(_ref, env);
				(vi.type_, phase)
			}
			ExprKind::New(new_expr) => {
				let NewExpr {
					class,
					obj_id,
					arg_list,
					obj_scope,
				} = new_expr;
				// Type check everything
				let class_type = self.type_check_exp(&class, env).0;
				let obj_scope_type = obj_scope.as_ref().map(|x| self.type_check_exp(x, env).0);
				let obj_id_type = obj_id.as_ref().map(|x| self.type_check_exp(x, env).0);
				let arg_list_types = self.type_check_arg_list(arg_list, env);

				let ExprKind::Reference(ref r) = class.kind else {
					self.spanned_error(exp,"Must be a reference to a class");
					return (self.types.error(), Phase::Independent);
				};

				let Reference::TypeReference(_) = r else {
					self.spanned_error(exp,"Must be a type reference to a class");
					return (self.types.error(), Phase::Independent);
				};

				// Lookup the class's type in the env
				let (class_env, class_symbol) = match *class_type {
					Type::Class(ref class) => {
						if class.phase == Phase::Independent || env.phase == class.phase {
							(&class.env, &class.name)
						} else {
							self.spanned_error(
								exp,
								format!(
									"Cannot create {} class \"{}\" in {} phase",
									class.phase, class.name, env.phase
								),
							);
							return (self.types.error(), Phase::Independent);
						}
					}
					// If type is anything we have to assume it's ok to initialize it
					Type::Anything => return (self.types.anything(), Phase::Independent),
					// If type is error, we assume the error was already reported and evauate the new expression to error as well
					Type::Unresolved => return self.resolved_error(),
					Type::Struct(_) => {
						self.spanned_error(
							class,
							format!("Cannot instantiate type \"{}\" because it is a struct and not a class. Use struct instantiation instead.", class_type),
						);
						return self.resolved_error();
					}
					_ => {
						self.spanned_error(
							class,
							format!("Cannot instantiate type \"{}\" because it is not a class", class_type),
						);
						return self.resolved_error();
					}
				};

				// Type check args against constructor
				let init_method_name = if env.phase == Phase::Preflight {
					CLASS_INIT_NAME
				} else {
					CLASS_INFLIGHT_INIT_NAME
				};

				let lookup_res = class_env.lookup_ext(&init_method_name.into(), None);
				let constructor_type = if let LookupResult::Found(k, _) = lookup_res {
					k.as_variable().expect("Expected constructor to be a variable").type_
				} else {
					self.type_error(lookup_result_to_type_error(
						lookup_res,
						&Symbol {
							name: CLASS_INIT_NAME.into(),
							span: class_symbol.span.clone(),
						},
					));
					return self.resolved_error();
				};
				let constructor_sig = constructor_type
					.as_function_sig()
					.expect("Expected constructor to be a function signature");

				// Verify return type (This should never fail since we define the constructors return type during AST building)
				self.validate_type(constructor_sig.return_type, class_type, exp);

				self.type_check_arg_list_against_function_sig(&arg_list, &constructor_sig, exp, arg_list_types);

				let non_std_args = !class_type.as_class().unwrap().std_construct_args;

				// If this is a preflight class make sure the object's scope and id are of correct type
				if class_type.is_preflight_class() {
					// Get reference to resource object's scope
					let obj_scope_type = if obj_scope_type.is_none() {
						// If this returns None, this means we're instantiating a preflight object in the global scope, which is valid
						env
							.lookup(&"this".into(), Some(self.statement_idx))
							.map(|v| v.as_variable().expect("Expected \"this\" to be a variable").type_)
					} else {
						// If this is a non-standard preflight class, make sure the object's scope isn't explicitly set (using the `in` keywords)
						if non_std_args {
							self.spanned_error(
								obj_scope.as_ref().unwrap(),
								format!(
									"Cannot set scope of non-standard preflight class \"{}\" using `in`",
									class_type
								),
							);
						}

						obj_scope_type
					};

					// Verify the object scope is an actually resource
					if let Some(obj_scope_type) = obj_scope_type {
						if !obj_scope_type.is_preflight_class() {
							self.spanned_error(
								exp,
								format!(
									"Expected scope to be a preflight object, instead found \"{}\"",
									obj_scope_type
								),
							);
						}
					}

					// Verify the object id is a string
					if let Some(obj_id_type) = obj_id_type {
						self.validate_type(obj_id_type, self.types.string(), obj_id.as_ref().unwrap());
						// If this is a non-standard preflight class, make sure the object's id isn't explicitly set (using the `as` keywords)
						if non_std_args {
							self.spanned_error(
								obj_id.as_ref().unwrap(),
								format!(
									"Cannot set id of non-standard preflight class \"{}\" using `as`",
									class_type
								),
							);
						}
					}
				} else {
					// This is an inflight class, make sure the object scope and id are not set
					if let Some(obj_scope) = obj_scope {
						self.spanned_error(obj_scope, "Inflight classes cannot have a scope");
					}
					if let Some(obj_id) = obj_id {
						self.spanned_error(obj_id, "Inflight classes cannot have an id");
					}
				}

				(class_type, env.phase)
			}
			ExprKind::Call { callee, arg_list } => {
				// Resolve the function's reference (either a method in the class's env or a function in the current env)
				let (func_type, callee_phase) = self.type_check_exp(callee, env);
				let is_option = func_type.is_option();
				let func_type = func_type.maybe_unwrap_option();

				let arg_list_types = self.type_check_arg_list(arg_list, env);

				// If the callee's signature type is unknown, just evaluate the entire call expression as an error
				if func_type.is_unresolved() {
					return self.resolved_error();
				}

				// If the caller's signature is `any`, then just evaluate the entire call expression as `any`
				if func_type.is_anything() {
					return (self.types.anything(), Phase::Independent);
				}

				// Make sure this is a function signature type
				let func_sig = if let Some(func_sig) = func_type.as_function_sig() {
					func_sig.clone()
				} else if let Some(class) = func_type.as_preflight_class() {
					// return the signature of the "handle" method
					let lookup_res = class.get_method(&CLOSURE_CLASS_HANDLE_METHOD.into());
					let handle_type = if let Some(method) = lookup_res {
						method.type_
					} else {
						self.spanned_error(callee, "Expected a function or method");
						return self.resolved_error();
					};
					if let Some(sig_type) = handle_type.as_function_sig() {
						sig_type.clone()
					} else {
						self.spanned_error(callee, "Expected a function or method");
						return self.resolved_error();
					}
				} else {
					self.spanned_error(
						callee,
						format!("Expected a function or method, found \"{}\"", func_type),
					);
					return self.resolved_error();
				};

				if !env.phase.can_call_to(&func_sig.phase) {
					self.spanned_error(
						exp,
						format!("Cannot call into {} phase while {}", func_sig.phase, env.phase),
					);
				}

				// if the function is phase independent, then inherit from the callee
				let func_phase = if func_sig.phase == Phase::Independent {
					callee_phase
				} else {
					func_sig.phase
				};

				if let Some(value) = self.type_check_arg_list_against_function_sig(arg_list, &func_sig, exp, arg_list_types) {
					return (value, func_phase);
				}

				// If the function is "wingc_env", then print out the current environment
				if let ExprKind::Reference(Reference::Identifier(ident)) = &callee.kind {
					if ident.name == "wingc_env" {
						println!("[symbol environment at {}]", exp.span().to_string());
						println!("{}", env.to_string());
					}
				}

				if is_option {
					// When calling a an optional function, the return type is always optional
					// To allow this to be both safe and unsurprising,
					// the callee must be a reference with an optional accessor
					if let ExprKind::Reference(Reference::InstanceMember { optional_accessor, .. }) = &callee.kind {
						if *optional_accessor {
							(self.types.make_option(func_sig.return_type), func_phase)
						} else {
							// No additional error is needed here, since the type checker will already have errored without optional chaining
							(self.types.error(), func_phase)
						}
					} else {
						// TODO do we want syntax for this? e.g. `foo?.()`
						self.spanned_error(callee, "Cannot call an optional function");
						(self.types.error(), func_phase)
					}
				} else {
					(func_sig.return_type, func_phase)
				}
			}
			ExprKind::ArrayLiteral { type_, items } => {
				// Infer type based on either the explicit type or the value in one of the items
				let container_type = if let Some(type_) = type_ {
					self.resolve_type_annotation(type_, env)
				} else if !items.is_empty() {
					let (some_val_type, _) = self.type_check_exp(items.iter().next().unwrap(), env);
					self.types.add_type(Type::Array(some_val_type))
				} else {
					if self.in_json > 0 {
						self.types.add_type(Type::Array(self.types.json()))
					} else {
						self.spanned_error(exp, "Cannot infer type of empty array");
						self.types.add_type(Type::Array(self.types.error()))
					}
				};

				let element_type = match *container_type {
					Type::Array(t) => t,
					Type::MutArray(t) => t,
					_ => {
						self.spanned_error(exp, format!("Expected \"Array\" type, found \"{}\"", container_type));
						self.types.error()
					}
				};

				// Verify all types are the same as the inferred type
				for v in items.iter() {
					let (t, _) = self.type_check_exp(v, env);
					self.check_json_serializable_or_validate_type(t, element_type, v);
				}

				(container_type, env.phase)
			}
			ExprKind::StructLiteral { type_, fields } => {
				// Find this struct's type in the environment
				let struct_type = self.resolve_type_annotation(type_, env);

				// Type check each of the struct's fields
				let field_types: IndexMap<Symbol, TypeRef> = fields
					.iter()
					.map(|(name, exp)| {
						let (t, _) = self.type_check_exp(exp, env);
						(name.clone(), t)
					})
					.collect();

				// If we don't have type information for the struct we don't need to validate the fields
				if struct_type.is_anything() || struct_type.is_unresolved() {
					return (struct_type, env.phase);
				}

				// Make sure it really is a struct type
				let st = struct_type
					.as_struct()
					.expect(&format!("Expected \"{}\" to be a struct type", struct_type));

				// Verify that all expected fields are present and are the right type
				for (name, kind, _info) in st.env.iter(true) {
					let field_type = kind
						.as_variable()
						.expect("Expected struct field to be a variable in the struct env")
						.type_;
					match fields.get(name.as_str()) {
						Some(field_exp) => {
							let t = field_types.get(name.as_str()).unwrap();
							self.validate_type(*t, field_type, field_exp);
						}
						None => {
							if !field_type.is_option() {
								self.spanned_error(exp, format!("\"{}\" is not initialized", name));
							}
						}
					}
				}

				// Verify that no unexpected fields are present
				for (name, _t) in field_types.iter() {
					if st.env.lookup(name, Some(self.statement_idx)).is_none() {
						self.spanned_error(exp, format!("\"{}\" is not a field of \"{}\"", name.name, st.name.name));
					}
				}

				(struct_type, env.phase)
			}
			ExprKind::JsonLiteral { is_mut, element } => {
				if *is_mut {
					self.is_in_mut_json = true;
				}

				self.in_json += 1;
				self.type_check_exp(&element, env);
				self.in_json -= 1;

				// When we are no longer in a Json literal, we reset the is_in_mut_json flag
				if self.in_json == 0 {
					self.is_in_mut_json = false;
				}

				if *is_mut {
					(self.types.mut_json(), env.phase)
				} else {
					(self.types.json(), env.phase)
				}
			}
			ExprKind::JsonMapLiteral { fields } => {
				fields.iter().for_each(|(_, v)| {
					let (t, _) = self.type_check_exp(v, env);
					// Ensure we dont allow MutJson to Json or vice versa
					match *t {
						Type::Json => {
							if self.is_in_mut_json {
								self.spanned_error(
									v,
									"Cannot assign type: \"Json\" to a \"MutJson\" field (hint: try using Json.deepMutCopy())"
										.to_string(),
								)
							}
						}
						Type::MutJson => {
							if !self.is_in_mut_json {
								self.spanned_error(
									v,
									"Cannot assign type: \"MutJson\" to a \"Json\" field (hint: try using Json.deepCopy())".to_string(),
								)
							}
						}
						_ => {}
					};

          if !t.is_json_legal_value() {
            self.spanned_error(
              v,
              format!(
                "Expected \"Json\" elements to be Json values (https://www.json.org/json-en.html), but got \"{}\" which is not a Json value",
                t
              ),
            );
          }
				});

				(self.types.json(), env.phase)
			}
			ExprKind::MapLiteral { fields, type_ } => {
				// Infer type based on either the explicit type or the value in one of the fields
				let container_type = if let Some(type_) = type_ {
					self.resolve_type_annotation(type_, env)
				} else if !fields.is_empty() {
					let (some_val_type, _) = self.type_check_exp(fields.iter().next().unwrap().1, env);
					self.types.add_type(Type::Map(some_val_type))
				} else {
					self.spanned_error(exp, "Cannot infer type of empty map");
					self.types.add_type(Type::Map(self.types.error()))
				};

				let value_type = match *container_type {
					Type::Map(t) => t,
					Type::MutMap(t) => t,
					_ => {
						self.spanned_error(exp, format!("Expected \"Map\" type, found \"{}\"", container_type));
						self.types.error()
					}
				};

				// Verify all types are the same as the inferred type
				for (_, v) in fields.iter() {
					let (t, _) = self.type_check_exp(v, env);
					self.validate_type(t, value_type, v);
				}

				(container_type, env.phase)
			}
			ExprKind::SetLiteral { type_, items } => {
				// Infer type based on either the explicit type or the value in one of the items
				let container_type = if let Some(type_) = type_ {
					self.resolve_type_annotation(type_, env)
				} else if !items.is_empty() {
					let (some_val_type, _) = self.type_check_exp(items.iter().next().unwrap(), env);
					self.types.add_type(Type::Set(some_val_type))
				} else {
					self.spanned_error(exp, "Cannot infer type of empty set");
					self.types.add_type(Type::Set(self.types.error()))
				};

				let element_type = match *container_type {
					Type::Set(t) => t,
					Type::MutSet(t) => t,
					_ => {
						self.spanned_error(exp, format!("Expected \"Set\" type, found \"{}\"", container_type));
						self.types.error()
					}
				};

				// Verify all types are the same as the inferred type
				for v in items.iter() {
					let (t, _) = self.type_check_exp(v, env);
					self.validate_type(t, element_type, v);
				}

				(container_type, env.phase)
			}
			ExprKind::FunctionClosure(func_def) => self.type_check_closure(func_def, env),
			ExprKind::CompilerDebugPanic => {
				// Handle the debug panic expression (during type-checking)
				dbg_panic!();
				(
					self.type_error(TypeError {
						message: "Panic expression".to_string(),
						span: exp.span.clone(),
					}),
					env.phase,
				)
			}
		}
	}

	fn resolved_error(&mut self) -> (UnsafeRef<Type>, Phase) {
		(self.types.error(), Phase::Independent)
	}

	fn type_check_arg_list_against_function_sig(
		&mut self,
		arg_list: &ArgList,
		func_sig: &FunctionSignature,
		exp: &impl Spanned,
		arg_list_types: ArgListTypes,
	) -> Option<UnsafeRef<Type>> {
		// Verify arity
		let pos_args_count = arg_list.pos_args.len();
		let min_args = func_sig.min_parameters();
		if pos_args_count < min_args {
			let err_text = format!(
				"Expected {} positional argument(s) but got {}",
				min_args, pos_args_count
			);
			self.spanned_error(exp, err_text);
			return Some(self.types.error());
		}

		if !arg_list.named_args.is_empty() {
			let last_arg = match func_sig.parameters.last() {
				Some(arg) => arg.typeref.maybe_unwrap_option(),
				None => {
					self.spanned_error(
						exp,
						format!("Expected 0 named arguments for func at {}", exp.span().to_string()),
					);
					return Some(self.types.error());
				}
			};

			if !last_arg.is_struct() {
				self.spanned_error(exp, "No named arguments expected");
				return Some(self.types.error());
			}

			self.validate_structural_type(&arg_list.named_args, &arg_list_types.named_args, &last_arg, exp);
		}

		// Count number of optional parameters from the end of the function's params
		// Allow arg_list to be missing up to that number of nil values to try and make the number of arguments match
		let num_optionals = func_sig
			.parameters
			.iter()
			.rev()
			.take_while(|arg| arg.typeref.is_option())
			.count();

		// Verify arity
		let arg_count = arg_list.pos_args.len() + (if arg_list.named_args.is_empty() { 0 } else { 1 });
		let min_args = func_sig.parameters.len() - num_optionals;
		let max_args = func_sig.parameters.len();
		if arg_count < min_args || arg_count > max_args {
			let err_text = if min_args == max_args {
				format!("Expected {} arguments but got {}", min_args, arg_count)
			} else {
				format!(
					"Expected between {} and {} arguments but got {}",
					min_args, max_args, arg_count
				)
			};
			self.spanned_error(exp, err_text);
		}
		let params = func_sig
			.parameters
			.iter()
			.take(func_sig.parameters.len() - num_optionals);

		// Verify passed positional arguments match the function's parameter types
		for (arg_expr, arg_type, param) in izip!(arg_list.pos_args.iter(), arg_list_types.pos_args.iter(), params) {
			self.validate_type(*arg_type, param.typeref, arg_expr);
		}
		None
	}

	fn type_check_closure(&mut self, func_def: &ast::FunctionDefinition, env: &SymbolEnv) -> (UnsafeRef<Type>, Phase) {
		// TODO: make sure this function returns on all control paths when there's a return type (can be done by recursively traversing the statements and making sure there's a "return" statements in all control paths)
		// https://github.com/winglang/wing/issues/457
		// Create a type_checker function signature from the AST function definition
		let function_type = self.resolve_type_annotation(&func_def.signature.to_type_annotation(), env);
		let sig = function_type.as_function_sig().unwrap();

		// Create an environment for the function
		let mut function_env = SymbolEnv::new(
			Some(env.get_ref()),
			sig.return_type,
			false,
			true,
			func_def.signature.phase,
			self.statement_idx,
		);
		self.add_arguments_to_env(&func_def.signature.parameters, &sig, &mut function_env);

		// Type check the function body
		if let FunctionBody::Statements(scope) = &func_def.body {
			scope.set_env(function_env);

			self.inner_scopes.push(scope);

			(function_type, sig.phase)
		} else {
			(function_type, sig.phase)
		}
	}

	/// Validate that a given map can be assigned to a variable of given struct type
	fn validate_structural_type(
		&mut self,
		object: &IndexMap<Symbol, Expr>,
		object_types: &IndexMap<Symbol, TypeRef>,
		expected_type: &TypeRef,
		value: &impl Spanned,
	) {
		let expected_struct = if let Some(expected_struct) = expected_type.as_struct() {
			expected_struct
		} else {
			self.spanned_error(value, "Named arguments provided for non-struct argument");
			return;
		};

		// Verify that there are no extraneous fields
		// Also map original field names to the ones in the struct type
		let mut field_map = IndexMap::new();
		for (k, _) in object_types.iter() {
			let field = expected_struct.env.lookup(k, None);
			if let Some(field) = field {
				let field_type = field
					.as_variable()
					.expect("Expected struct field to be a variable in the struct env")
					.type_;
				field_map.insert(k.name.clone(), (k, field_type));
			} else {
				self.spanned_error(value, format!("\"{}\" is not a field of \"{}\"", k.name, expected_type));
			}
		}

		// Verify that all non-optional fields are present and are of the right type
		for (k, v) in expected_struct.env.iter(true).map(|(k, v, _)| {
			(
				k,
				v.as_variable()
					.expect("Expected struct field to be a variable in the struct env")
					.type_,
			)
		}) {
			if let Some((symb, expected_field_type)) = field_map.get(&k) {
				let provided_exp = object.get(*symb).unwrap();
				let t = object_types.get(*symb).unwrap();
				self.validate_type(*t, *expected_field_type, provided_exp);
			} else if !v.is_option() {
				self.spanned_error(
					value,
					format!(
						"Missing required field \"{}\" from \"{}\"",
						k, expected_struct.name.name
					),
				);
			}
		}
	}

	fn check_json_serializable_or_validate_type(
		&mut self,
		actual_type: TypeRef,
		expected_type: TypeRef,
		exp: &Expr,
	) -> TypeRef {
		// Skip validate if in Json
		if self.in_json == 0 {
			return self.validate_type(actual_type, expected_type, exp);
		}

		if !actual_type.is_json_legal_value() {
			self.spanned_error(
				exp,
				format!(
					"Expected \"Json\" elements to be Json values (https://www.json.org/json-en.html), but got \"{}\" which is not a Json value",
					actual_type
				),
			);
			return self.types.error();
		}

		actual_type
	}

	/// Validate that the given type is a subtype (or same) as the expected type. If not, add an error
	/// to the diagnostics.
	/// Returns the given type on success, otherwise returns the expected type.
	fn validate_type(&mut self, actual_type: TypeRef, expected_type: TypeRef, span: &impl Spanned) -> TypeRef {
		self.validate_type_in(actual_type, &[expected_type], span)
	}

	/// Validate that the given type is a subtype (or same) as the one of the expected types. If not, add
	/// an error to the diagnostics.
	/// Returns the given type on success, otherwise returns one of the expected types.
	fn validate_type_in(&mut self, actual_type: TypeRef, expected_types: &[TypeRef], span: &impl Spanned) -> TypeRef {
		assert!(expected_types.len() > 0);

		// If the actual type is anything or any of the expected types then we're good
		if actual_type.is_anything() || expected_types.iter().any(|t| actual_type.is_subtype_of(t)) {
			return actual_type;
		}

		// If the actual type is an error (a type we failed to resolve) then we silently ignore it assuming
		// the error was already reported.
		if actual_type.is_unresolved() {
			return actual_type;
		}

		// If any of the expected types are errors (types we failed to resolve) then we silently ignore it
		// assuming the error was already reported.
		if expected_types.iter().any(|t| t.is_unresolved()) {
			return actual_type;
		}

		// If the expected type is Json and the actual type is a Json legal value then we're good
		if expected_types.iter().any(|t| t.is_json()) {
			if actual_type.is_json_legal_value() {
				return actual_type;
			}
		}

		let expected_type_str = if expected_types.len() > 1 {
			let expected_types_list = expected_types
				.iter()
				.map(|t| format!("{}", t))
				.collect::<Vec<String>>()
				.join(",");
			format!("one of \"{}\"", expected_types_list)
		} else {
			format!("\"{}\"", expected_types[0])
		};

		let mut message = format!(
			"Expected type to be {}, but got \"{}\" instead",
			expected_type_str, actual_type
		);
		if actual_type.is_nil() && expected_types.len() == 1 {
			message = format!(
				"{} (hint: to allow \"nil\" assignment use optional type: \"{}?\")",
				message, expected_types[0]
			);
		}
		report_diagnostic(Diagnostic {
			message,
			span: Some(span.span()),
		});

		// Evaluate to one of the expected types
		expected_types[0]
	}

	pub fn type_check_scope(&mut self, scope: &Scope) {
		CompilationContext::set(CompilationPhase::TypeChecking, &scope.span);
		assert!(self.inner_scopes.is_empty());
		for statement in scope.statements.iter() {
			self.type_check_statement(statement, scope.env.borrow_mut().as_mut().unwrap());
		}
		let inner_scopes = self.inner_scopes.drain(..).collect::<Vec<_>>();
		for inner_scope in inner_scopes {
			self.type_check_scope(unsafe { &*inner_scope });
		}
	}

	fn resolve_type_annotation(&mut self, annotation: &TypeAnnotation, env: &SymbolEnv) -> TypeRef {
		match &annotation.kind {
			TypeAnnotationKind::Number => self.types.number(),
			TypeAnnotationKind::String => self.types.string(),
			TypeAnnotationKind::Bool => self.types.bool(),
			TypeAnnotationKind::Duration => self.types.duration(),
			TypeAnnotationKind::Void => self.types.void(),
			TypeAnnotationKind::Json => self.types.json(),
			TypeAnnotationKind::MutJson => self.types.mut_json(),
			TypeAnnotationKind::Optional(v) => {
				let value_type = self.resolve_type_annotation(v, env);
				self.types.add_type(Type::Optional(value_type))
			}
			TypeAnnotationKind::Function(ast_sig) => {
				let mut parameters = vec![];
				for p in ast_sig.parameters.iter() {
					parameters.push(FunctionParameter {
						name: p.name.name.clone(),
						typeref: self.resolve_type_annotation(&p.type_annotation, env),
						docs: Docs::default(),
					});
				}
				let sig = FunctionSignature {
					this_type: None,
					parameters,
					return_type: self.resolve_type_annotation(ast_sig.return_type.as_ref(), env),
					phase: ast_sig.phase,
					js_override: None,
					docs: Docs::default(),
				};
				// TODO: avoid creating a new type for each function_sig resolution
				self.types.add_type(Type::Function(sig))
			}
			TypeAnnotationKind::UserDefined(user_defined_type) => self
				.resolve_user_defined_type(user_defined_type, env, self.statement_idx)
				.unwrap_or_else(|e| self.type_error(e)),
			TypeAnnotationKind::Array(v) => {
				let value_type = self.resolve_type_annotation(v, env);
				// TODO: avoid creating a new type for each array resolution
				self.types.add_type(Type::Array(value_type))
			}
			TypeAnnotationKind::MutArray(v) => {
				let value_type = self.resolve_type_annotation(v, env);
				// TODO: avoid creating a new type for each array resolution
				self.types.add_type(Type::MutArray(value_type))
			}
			TypeAnnotationKind::Set(v) => {
				let value_type = self.resolve_type_annotation(v, env);
				// TODO: avoid creating a new type for each set resolution
				self.types.add_type(Type::Set(value_type))
			}
			TypeAnnotationKind::MutSet(v) => {
				let value_type = self.resolve_type_annotation(v, env);
				// TODO: avoid creating a new type for each set resolution
				self.types.add_type(Type::MutSet(value_type))
			}
			TypeAnnotationKind::Map(v) => {
				let value_type = self.resolve_type_annotation(v, env);
				// TODO: avoid creating a new type for each map resolution
				self.types.add_type(Type::Map(value_type))
			}
			TypeAnnotationKind::MutMap(v) => {
				let value_type = self.resolve_type_annotation(v, env);
				// TODO: avoid creating a new type for each map resolution
				self.types.add_type(Type::MutMap(value_type))
			}
		}
	}

	fn type_check_arg_list(&mut self, arg_list: &ArgList, env: &SymbolEnv) -> ArgListTypes {
		// Type check the positional arguments, e.g. fn(exp1, exp2, exp3)
		let pos_arg_types = arg_list
			.pos_args
			.iter()
			.map(|pos_arg| self.type_check_exp(pos_arg, env).0)
			.collect();

		// Type check the named arguments, e.g. fn(named_arg1: exp4, named_arg2: exp5)
		let named_arg_types = arg_list
			.named_args
			.iter()
			.map(|(sym, expr)| {
				let arg_type = self.type_check_exp(&expr, env).0;
				(sym.clone(), arg_type)
			})
			.collect::<IndexMap<_, _>>();

		ArgListTypes {
			pos_args: pos_arg_types,
			named_args: named_arg_types,
		}
	}
	fn type_check_statement(&mut self, stmt: &Stmt, env: &mut SymbolEnv) {
		CompilationContext::set(CompilationPhase::TypeChecking, &stmt.span);

		// Set the current statement index for symbol lookup checks. We can safely assume we're
		// not overwriting the current statement index because `type_check_statement` is never
		// recursively called (we use a breadth-first traversal of the AST statements).
		self.statement_idx = stmt.idx;

		match &stmt.kind {
			StmtKind::Let {
				reassignable,
				var_name,
				initial_value,
				type_,
			} => {
				let explicit_type = type_.as_ref().map(|t| self.resolve_type_annotation(t, env));
				let (inferred_type, _) = self.type_check_exp(initial_value, env);
				if inferred_type.is_void() {
					self.spanned_error(
						var_name,
						format!("Cannot assign expression of type \"{}\" to a variable", inferred_type),
					);
				}
				if explicit_type.is_none() && inferred_type.is_nil() {
					self.spanned_error(
						initial_value,
						"Cannot assign nil value to variables without explicit optional type",
					);
				}
				if let Some(explicit_type) = explicit_type {
					self.validate_type(inferred_type, explicit_type, initial_value);
					match env.define(
						var_name,
						SymbolKind::make_free_variable(var_name.clone(), explicit_type, *reassignable, env.phase),
						StatementIdx::Index(stmt.idx),
					) {
						Err(type_error) => {
							self.type_error(type_error);
						}
						_ => {}
					};
				} else {
					match env.define(
						var_name,
						SymbolKind::make_free_variable(var_name.clone(), inferred_type, *reassignable, env.phase),
						StatementIdx::Index(stmt.idx),
					) {
						Err(type_error) => {
							self.type_error(type_error);
						}
						_ => {}
					};
				}
			}
			StmtKind::ForLoop {
				iterator,
				iterable,
				statements,
			} => {
				// TODO: Expression must be iterable
				let (exp_type, _) = self.type_check_exp(iterable, env);

				if !exp_type.is_iterable() {
					self.spanned_error(iterable, format!("Unable to iterate over \"{}\"", &exp_type));
				}

				let iterator_type = match &*exp_type {
					// These are builtin iterables that have a clear/direct iterable type
					Type::Array(t) => *t,
					Type::Set(t) => *t,
					Type::MutArray(t) => *t,
					Type::MutSet(t) => *t,
					Type::Anything => exp_type,
					_t => self.types.error(),
				};

				let mut scope_env = SymbolEnv::new(Some(env.get_ref()), env.return_type, false, false, env.phase, stmt.idx);
				match scope_env.define(
					&iterator,
					SymbolKind::make_free_variable(iterator.clone(), iterator_type, false, env.phase),
					StatementIdx::Top,
				) {
					Err(type_error) => {
						self.type_error(type_error);
					}
					_ => {}
				};
				statements.set_env(scope_env);

				self.inner_scopes.push(statements);
			}
			StmtKind::While { condition, statements } => {
				let (cond_type, _) = self.type_check_exp(condition, env);
				self.validate_type(cond_type, self.types.bool(), condition);

				statements.set_env(SymbolEnv::new(
					Some(env.get_ref()),
					env.return_type,
					false,
					false,
					env.phase,
					stmt.idx,
				));

				self.inner_scopes.push(statements);
			}
			StmtKind::Break | StmtKind::Continue => {}
			StmtKind::IfLet {
				value,
				statements,
				var_name,
				else_statements,
			} => {
				let (cond_type, _) = self.type_check_exp(value, env);

				if !cond_type.is_option() {
					report_diagnostic(Diagnostic {
						message: format!("Expected type to be optional, but got \"{}\" instead", cond_type),
						span: Some(value.span()),
					});
				}

				// Technically we only allow if let statements to be used with optionals
				// and above validate_type_is_optional method will attach a diagnostic error if it is not.
				// However for the sake of verbose diagnostics we'll allow the code to continue if the type is not an optional
				// and complete the type checking process for additional errors.
				let var_type = *cond_type.maybe_unwrap_option();

				let mut stmt_env = SymbolEnv::new(Some(env.get_ref()), env.return_type, false, false, env.phase, stmt.idx);

				// Add the variable to if block scope
				match stmt_env.define(
					var_name,
					SymbolKind::make_free_variable(var_name.clone(), var_type, false, env.phase),
					StatementIdx::Top,
				) {
					Err(type_error) => {
						self.type_error(type_error);
					}
					_ => {}
				}

				statements.set_env(stmt_env);
				self.inner_scopes.push(statements);

				if let Some(else_scope) = else_statements {
					else_scope.set_env(SymbolEnv::new(
						Some(env.get_ref()),
						env.return_type,
						false,
						false,
						env.phase,
						stmt.idx,
					));
					self.inner_scopes.push(else_scope);
				}
			}
			StmtKind::If {
				condition,
				statements,
				elif_statements,
				else_statements,
			} => {
				let (cond_type, _) = self.type_check_exp(condition, env);
				self.validate_type(cond_type, self.types.bool(), condition);

				statements.set_env(SymbolEnv::new(
					Some(env.get_ref()),
					env.return_type,
					false,
					false,
					env.phase,
					stmt.idx,
				));
				self.inner_scopes.push(statements);

				for elif_scope in elif_statements {
					let (cond_type, _) = self.type_check_exp(&elif_scope.condition, env);
					self.validate_type(cond_type, self.types.bool(), condition);

					(&elif_scope.statements).set_env(SymbolEnv::new(
						Some(env.get_ref()),
						env.return_type,
						false,
						false,
						env.phase,
						stmt.idx,
					));
					self.inner_scopes.push(&elif_scope.statements);
				}

				if let Some(else_scope) = else_statements {
					else_scope.set_env(SymbolEnv::new(
						Some(env.get_ref()),
						env.return_type,
						false,
						false,
						env.phase,
						stmt.idx,
					));
					self.inner_scopes.push(else_scope);
				}
			}
			StmtKind::Expression(e) => {
				self.type_check_exp(e, env);
			}
			StmtKind::Assignment { variable, value } => {
				let (exp_type, _) = self.type_check_exp(value, env);
				let (var_type, var_phase) = self.type_check_exp(variable, env);

				// TODO: we need to verify that if this variable is defined in a parent environment (i.e.
				// being captured) it cannot be reassigned: https://github.com/winglang/wing/issues/3069

				if let ExprKind::Reference(r) = &variable.kind {
					let (var, _) = self.resolve_reference(&r, env);

					if !var_type.is_unresolved() && !var.reassignable {
						self.spanned_error(variable, "Variable is not reassignable".to_string());
					} else if var_phase == Phase::Preflight && env.phase == Phase::Inflight {
						self.spanned_error(stmt, "Variable cannot be reassigned from inflight".to_string());
					}
				}

				self.validate_type(exp_type, var_type, value);
			}
			StmtKind::Bring {
				module_name,
				identifier,
			} => {
				// library_name is the name of the library we are importing from the JSII world
				let library_name: String;
				// namespace_filter describes what types we are importing from the library
				// e.g. [] means we are importing everything from `mylib`
				// e.g. ["ns1", "ns2"] means we are importing everything from `mylib.ns1.ns2`
				let namespace_filter: Vec<String>;
				// alias is the symbol we are giving to the imported library or namespace
				let alias: &Symbol;

				if module_name.name.starts_with('"') && module_name.name.ends_with('"') {
					// case 1: bring "library_name" as identifier;
					if identifier.is_none() {
						self.spanned_error(
							stmt,
							format!(
								"bring {} must be assigned to an identifier (e.g. bring \"foo\" as foo)",
								module_name.name
							),
						);
						return;
					}
					// We assume we have a jsii library and we use `module_name` as the library name, and set no
					// namespace filter (we only support importing a full library at the moment)
					library_name = module_name.name[1..module_name.name.len() - 1].to_string();
					namespace_filter = vec![];
					alias = identifier.as_ref().unwrap();
				} else {
					// case 2: bring module_name;
					// case 3: bring module_name as identifier;
					if WINGSDK_BRINGABLE_MODULES.contains(&module_name.name.as_str()) {
						library_name = WINGSDK_ASSEMBLY_NAME.to_string();
						namespace_filter = vec![module_name.name.clone()];
						alias = identifier.as_ref().unwrap_or(&module_name);
					} else if module_name.name.as_str() == WINGSDK_STD_MODULE {
						self.spanned_error(stmt, format!("Redundant bring of \"{}\"", WINGSDK_STD_MODULE));
						return;
					} else {
						self.spanned_error(stmt, format!("\"{}\" is not a built-in module", module_name.name));
						return;
					}
				};

				self.add_module_to_env(env, library_name, namespace_filter, &alias, Some(&stmt));
			}
			StmtKind::Scope(scope) => {
				scope.set_env(SymbolEnv::new(
					Some(env.get_ref()),
					env.return_type,
					false,
					false,
					env.phase,
					stmt.idx,
				));
				self.inner_scopes.push(scope)
			}
			StmtKind::Return(exp) => {
				if let Some(return_expression) = exp {
					let (return_type, _) = self.type_check_exp(return_expression, env);
					if !env.return_type.is_void() {
						self.validate_type(return_type, env.return_type, return_expression);
					} else if env.is_in_function() {
						self.spanned_error(stmt, "Unexpected return value from void function");
					} else {
						self.spanned_error(stmt, "Return statement outside of function cannot return a value");
					}
				} else {
					if !env.return_type.is_void() {
						self.spanned_error(
							stmt,
							format!("Expected return statement to return type {}", env.return_type),
						);
					}
				}
			}
			StmtKind::Class(AstClass {
				name,
				fields,
				methods,
				parent,
				implements,
				initializer,
				phase,
				inflight_initializer,
			}) => {
				// preflight classes cannot be declared inside an inflight scope
				// (the other way is okay)
				if env.phase == Phase::Inflight && *phase == Phase::Preflight {
					self.spanned_error(stmt, format!("Cannot declare a {} class in {} scope", phase, env.phase));
				}
				// Verify parent is a known class and get their env
				let (parent_class, parent_class_env) = self.extract_parent_class(parent.as_ref(), *phase, name, env);

				// Create environment representing this class, for now it'll be empty just so we can support referencing ourselves from the class definition.
				let dummy_env = SymbolEnv::new(None, self.types.void(), false, false, env.phase, stmt.idx);

				let impl_interfaces = implements
					.iter()
					.filter_map(|i| {
						let t = self
							.resolve_user_defined_type(i, env, stmt.idx)
							.unwrap_or_else(|e| self.type_error(e));
						if t.as_interface().is_some() {
							Some(t)
						} else {
							self.spanned_error(i, format!("Expected an interface, instead found type \"{}\"", t));
							None
						}
					})
					.collect::<Vec<_>>();

				// Create the resource/class type and add it to the current environment (so class implementation can reference itself)
				let class_spec = Class {
					name: name.clone(),
					fqn: None,
					env: dummy_env,
					parent: parent_class,
					implements: impl_interfaces.clone(),
					is_abstract: false,
					phase: *phase,
					type_parameters: None, // TODO no way to have generic args in wing yet
					docs: Docs::default(),
					std_construct_args: *phase == Phase::Preflight,
					lifts: None,
				};
				let mut class_type = self.types.add_type(Type::Class(class_spec));
				match env.define(name, SymbolKind::Type(class_type), StatementIdx::Top) {
					Err(type_error) => {
						self.type_error(type_error);
					}
					_ => {}
				};

				// Create a the real class environment to be filled with the class AST types
				let mut class_env = SymbolEnv::new(parent_class_env, self.types.void(), false, false, env.phase, stmt.idx);

				// Add fields to the class env
				for field in fields.iter() {
					let field_type = self.resolve_type_annotation(&field.member_type, env);
					match class_env.define(
						&field.name,
						SymbolKind::make_member_variable(
							field.name.clone(),
							field_type,
							field.reassignable,
							field.is_static,
							field.phase,
							None,
						),
						StatementIdx::Top,
					) {
						Err(type_error) => {
							self.type_error(type_error);
						}
						_ => {}
					};
				}

				// Add methods to the class env
				for (method_name, method_def) in methods.iter() {
					self.add_method_to_class_env(
						&method_def.signature,
						env,
						if method_def.is_static { None } else { Some(class_type) },
						&mut class_env,
						method_name,
					);
				}

				// Add the constructor to the class env
				let init_symb = Symbol {
					name: CLASS_INIT_NAME.into(),
					span: initializer.span.clone(),
				};

				self.add_method_to_class_env(&initializer.signature, env, None, &mut class_env, &init_symb);

				let inflight_init_symb = Symbol {
					name: CLASS_INFLIGHT_INIT_NAME.into(),
					span: inflight_initializer.span.clone(),
				};

				// Add the inflight initializer to the class env
				self.add_method_to_class_env(
					&inflight_initializer.signature,
					env,
					Some(class_type),
					&mut class_env,
					&inflight_init_symb,
				);

				// Replace the dummy class environment with the real one before type checking the methods
				class_type.as_class_mut().unwrap().env = class_env;
				let class_env = &class_type.as_class().unwrap().env;

				if let FunctionBody::Statements(scope) = &inflight_initializer.body {
					self.check_class_field_initialization(&scope, fields, Phase::Inflight);
					self.type_check_super_constructor_against_parent_initializer(
						scope,
						class_type,
						&class_env,
						CLASS_INFLIGHT_INIT_NAME,
					);
				};

				// Type check constructor
				self.type_check_method(class_env, &init_symb, env, stmt.idx, initializer, class_type);

				// Verify if all fields of a class/resource are initialized in the initializer.
				let init_statements = match &initializer.body {
					FunctionBody::Statements(s) => s,
					FunctionBody::External(_) => panic!("init cannot be extern"),
				};

				self.check_class_field_initialization(&init_statements, fields, Phase::Preflight);

				self.type_check_super_constructor_against_parent_initializer(
					init_statements,
					class_type,
					&class_env,
					CLASS_INIT_NAME,
				);

				// Type check the inflight initializer
				self.type_check_method(
					class_env,
					&inflight_init_symb,
					env,
					stmt.idx,
					inflight_initializer,
					class_type,
				);

				// TODO: handle member/method overrides in our env based on whatever rules we define in our spec
				// https://github.com/winglang/wing/issues/1124

				// Type check methods
				for (method_name, method_def) in methods.iter() {
					self.type_check_method(class_env, method_name, env, stmt.idx, method_def, class_type);
				}

				// Check that the class satisfies all of its interfaces
				for interface_type in impl_interfaces.iter() {
					let interface_type = match interface_type.as_interface() {
						Some(t) => t,
						None => {
							// No need to error here, it will be caught when `impl_interaces` was created
							continue;
						}
					};

					// Check all methods are implemented
					for (method_name, method_type) in interface_type.methods(true) {
						if let Some(symbol) = class_env.lookup(&method_name.as_str().into(), None) {
							let class_method_type = symbol.as_variable().expect("Expected method to be a variable").type_;
							self.validate_type(class_method_type, method_type, name);
						} else {
							self.spanned_error(
								name,
								format!(
									"Class \"{}\" does not implement method \"{}\" of interface \"{}\"",
									name.name, method_name, interface_type.name.name
								),
							);
						}
					}

					// Check all fields are implemented
					for (field_name, field_type) in interface_type.fields(true) {
						if let Some(symbol) = class_env.lookup(&field_name.as_str().into(), None) {
							let class_field_type = symbol.as_variable().expect("Expected field to be a variable").type_;
							self.validate_type(class_field_type, field_type, name);
						} else {
							self.spanned_error(
								name,
								format!(
									"Class \"{}\" does not implement field \"{}\" of interface \"{}\"",
									name.name, field_name, interface_type.name.name
								),
							);
						}
					}
				}
			}
			StmtKind::Interface(AstInterface { name, methods, extends }) => {
				// Create environment representing this interface, for now it'll be empty just so we can support referencing ourselves from the interface definition.
				let dummy_env = SymbolEnv::new(None, self.types.void(), false, false, env.phase, stmt.idx);

				let extend_interfaces = extends
					.iter()
					.filter_map(|i| {
						let t = self
							.resolve_user_defined_type(i, env, stmt.idx)
							.unwrap_or_else(|e| self.type_error(e));
						if t.as_interface().is_some() {
							Some(t)
						} else {
							// The type checker resolves non-existing definitions to `any`, so we avoid duplicate errors by checking for that here
							if !t.is_unresolved() {
								self.spanned_error(i, format!("Expected an interface, instead found type \"{}\"", t));
							}
							None
						}
					})
					.collect::<Vec<_>>();

				// Create the interface type and add it to the current environment (so interface implementation can reference itself)
				let interface_spec = Interface {
					name: name.clone(),
					docs: Docs::default(),
					env: dummy_env,
					extends: extend_interfaces.clone(),
				};
				let mut interface_type = self.types.add_type(Type::Interface(interface_spec));
				match env.define(name, SymbolKind::Type(interface_type), StatementIdx::Top) {
					Err(type_error) => {
						self.type_error(type_error);
					}
					_ => {}
				};

				// Create the real interface environment to be filled with the interface AST types
				let mut interface_env = SymbolEnv::new(None, self.types.void(), false, false, env.phase, stmt.idx);

				// Add methods to the interface env
				for (method_name, sig) in methods.iter() {
					let mut method_type = self.resolve_type_annotation(&sig.to_type_annotation(), env);
					// use the interface type as the function's "this" type
					if let Type::Function(ref mut f) = *method_type {
						f.this_type = Some(interface_type);
					} else {
						panic!("Expected method type to be a function");
					}

					match interface_env.define(
						method_name,
						SymbolKind::make_member_variable(method_name.clone(), method_type, false, false, sig.phase, None),
						StatementIdx::Top,
					) {
						Err(type_error) => {
							self.type_error(type_error);
						}
						_ => {}
					};
				}

				// add methods from all extended interfaces to the interface env
				if let Err(e) = add_parent_members_to_iface_env(&extend_interfaces, name, &mut interface_env) {
					self.type_error(e);
				}

				// Replace the dummy interface environment with the real one before type checking the methods
				interface_type.as_mut_interface().unwrap().env = interface_env;
			}
			StmtKind::Struct { name, extends, fields } => {
				// Note: structs don't have a parent environment, instead they flatten their parent's members into the struct's env.
				//   If we encounter an existing member with the same name and type we skip it, if the types are different we
				//   fail type checking.

				// Create an environment for the struct
				let mut struct_env = SymbolEnv::new(None, self.types.void(), false, false, env.phase, stmt.idx);

				// Add fields to the struct env
				for field in fields.iter() {
					let field_type = self.resolve_type_annotation(&field.member_type, env);
					if field_type.is_mutable() {
						self.spanned_error(&field.name, "Struct fields must have immutable types");
					}
					match struct_env.define(
						&field.name,
						SymbolKind::make_member_variable(field.name.clone(), field_type, false, false, Phase::Independent, None),
						StatementIdx::Top,
					) {
						Err(type_error) => {
							self.type_error(type_error);
						}
						_ => {}
					};
				}

				// Add members from the structs parents
				let extends_types = extends
					.iter()
					.filter_map(|ext| {
						let t = self
							.resolve_user_defined_type(ext, env, stmt.idx)
							.unwrap_or_else(|e| self.type_error(e));
						if t.as_struct().is_some() {
							Some(t)
						} else {
							self.spanned_error(ext, format!("Expected a struct, found type \"{}\"", t));
							None
						}
					})
					.collect::<Vec<_>>();

				if let Err(e) = add_parent_members_to_struct_env(&extends_types, name, &mut struct_env) {
					self.type_error(e);
				}
				match env.define(
					name,
					SymbolKind::Type(self.types.add_type(Type::Struct(Struct {
						name: name.clone(),
						extends: extends_types,
						env: struct_env,
						docs: Docs::default(),
					}))),
					StatementIdx::Top,
				) {
					Err(type_error) => {
						self.type_error(type_error);
					}
					_ => {}
				};
			}
			StmtKind::Enum { name, values } => {
				let enum_type_ref = self.types.add_type(Type::Enum(Enum {
					name: name.clone(),
					values: values.clone(),
					docs: Default::default(),
				}));

				match env.define(name, SymbolKind::Type(enum_type_ref), StatementIdx::Top) {
					Err(type_error) => {
						self.type_error(type_error);
					}
					_ => {}
				};
			}
			StmtKind::TryCatch {
				try_statements,
				catch_block,
				finally_statements,
			} => {
				// Create a new environment for the try block
				let try_env = SymbolEnv::new(Some(env.get_ref()), env.return_type, false, false, env.phase, stmt.idx);
				try_statements.set_env(try_env);
				self.inner_scopes.push(try_statements);

				// Create a new environment for the catch block
				if let Some(catch_block) = catch_block {
					let mut catch_env = SymbolEnv::new(Some(env.get_ref()), env.return_type, false, false, env.phase, stmt.idx);

					// Add the exception variable to the catch block
					if let Some(exception_var) = &catch_block.exception_var {
						match catch_env.define(
							exception_var,
							SymbolKind::make_free_variable(exception_var.clone(), self.types.string(), false, env.phase),
							StatementIdx::Top,
						) {
							Err(type_error) => {
								self.type_error(type_error);
							}
							_ => {}
						}
					}
					catch_block.statements.set_env(catch_env);
					self.inner_scopes.push(&catch_block.statements);
				}

				// Create a new environment for the finally block
				if let Some(finally_statements) = finally_statements {
					let finally_env = SymbolEnv::new(Some(env.get_ref()), env.return_type, false, false, env.phase, stmt.idx);
					finally_statements.set_env(finally_env);
					self.inner_scopes.push(finally_statements);
				}
			}
			StmtKind::CompilerDebugEnv => {
				println!("[symbol environment at {}]", stmt.span);
				println!("{}", env);
			}
			StmtKind::SuperConstructor { arg_list } => {
				self.type_check_arg_list(arg_list, env);
			}
		}
	}

	fn type_check_super_constructor_against_parent_initializer(
		&mut self,
		scope: &Scope,
		class_type: UnsafeRef<Type>,
		class_env: &SymbolEnv,
		init_name: &str,
	) {
		if &scope.statements.len() >= &1 {
			match &scope.statements[0].kind {
				StmtKind::SuperConstructor { arg_list } => {
					if let Some(parent_class) = &class_type.as_class().unwrap().parent {
						let parent_initializer = parent_class
							.as_class()
							.unwrap()
							.methods(false)
							.filter(|(name, _type)| name == init_name)
							.collect_vec()[0]
							.1;

						let class_initializer = &class_type
							.as_class()
							.unwrap()
							.methods(true)
							.filter(|(name, _type)| name == init_name)
							.collect_vec()[0]
							.1;

						// Create a temp init environment to use for typechecking args
						let mut init_env = SymbolEnv::new(
							Some(class_env.get_ref()),
							self.types.void(),
							true,
							true,
							class_env.phase,
							scope.statements[0].idx,
						);

						// add the initializer args to the init_env
						for arg in class_initializer.as_function_sig().unwrap().parameters.iter() {
							let sym = Symbol {
								name: arg.name.clone(),
								span: scope.statements[0].span.clone(),
							};
							match init_env.define(
								&sym,
								SymbolKind::make_free_variable(sym.clone(), arg.typeref, false, init_env.phase),
								StatementIdx::Top,
							) {
								Err(type_error) => {
									self.type_error(type_error);
								}
								_ => {}
							};
						}

						let arg_list_types = self.type_check_arg_list(&arg_list, &init_env);
						self.type_check_arg_list_against_function_sig(
							&arg_list,
							parent_initializer.as_function_sig().unwrap(),
							&scope.statements[0],
							arg_list_types,
						);
					}
				}
				_ => {} // No super no problem
			}
		};
	}

	/// Validate if the fields of a class are initialized in the constructor (init) according to the given phase.
	/// For example, if the phase is preflight, then all non-static preflight fields must be initialized
	/// and if the phase is inflight, then all non-static inflight fields must be initialized.
	///
	/// # Arguments
	///
	/// * `scope` - The constructor scope (init)
	/// * `fields` - All fields of a class
	/// * `phase` - initializer phase
	///
	fn check_class_field_initialization(&mut self, scope: &Scope, fields: &[ClassField], phase: Phase) {
		let mut visit_init = VisitClassInit::default();
		visit_init.analyze_statements(&scope.statements);
		let initialized_fields = visit_init.fields;

		let (current_phase, forbidden_phase) = if phase == Phase::Inflight {
			("Inflight", Phase::Preflight)
		} else {
			("Preflight", Phase::Inflight)
		};

		for field in fields.iter() {
			let matching_field = initialized_fields.iter().find(|&s| &s.name == &field.name.name);
			// inflight or static fields cannot be initialized in the initializer
			if field.phase == forbidden_phase || field.is_static {
				if let Some(matching_field) = matching_field {
					self.spanned_error(
						matching_field,
						format!(
							"\"{}\" cannot be initialized in the {} initializer",
							matching_field.name,
							current_phase.to_lowercase()
						),
					);
				};
				continue;
			}

			if matching_field == None {
				self.spanned_error(
					&field.name,
					format!("{} field \"{}\" is not initialized", current_phase, field.name.name),
				);
			}
		}
	}

	fn type_check_method(
		&mut self,
		class_env: &SymbolEnv,
		method_name: &Symbol,
		parent_env: &SymbolEnv, // the environment in which the class is declared
		statement_idx: usize,
		method_def: &FunctionDefinition,
		class_type: UnsafeRef<Type>,
	) {
		// TODO: make sure this function returns on all control paths when there's a return type (can be done by recursively traversing the statements and making sure there's a "return" statements in all control paths)
		// https://github.com/winglang/wing/issues/457
		// Lookup the method in the class_env
		let method_type = class_env
			.lookup(&method_name, None)
			.expect(format!("Expected method '{}' to be in class env", method_name.name).as_str())
			.as_variable()
			.expect("Expected method to be a variable")
			.type_;

		let method_sig = method_type
			.as_function_sig()
			.expect("Expected method type to be a function signature");

		// Create method environment and prime it with args
		let is_init = method_name.name == CLASS_INIT_NAME || method_name.name == CLASS_INFLIGHT_INIT_NAME;
		let mut method_env = SymbolEnv::new(
			Some(parent_env.get_ref()),
			method_sig.return_type,
			is_init,
			true,
			method_sig.phase,
			statement_idx,
		);
		// Prime the method environment with `this`
		if !method_def.is_static || is_init {
			method_env
				.define(
					&Symbol {
						name: "this".into(),
						span: method_name.span.clone(),
					},
					SymbolKind::make_free_variable("this".into(), class_type, false, class_env.phase),
					StatementIdx::Top,
				)
				.expect("Expected `this` to be added to constructor env");
		}
		self.add_arguments_to_env(&method_def.signature.parameters, method_sig, &mut method_env);

		if let FunctionBody::Statements(scope) = &method_def.body {
			scope.set_env(method_env);
			self.inner_scopes.push(scope);
		}
	}

	fn add_method_to_class_env(
		&mut self,
		method_sig: &ast::FunctionSignature,
		env: &mut SymbolEnv,
		instance_type: Option<TypeRef>,
		class_env: &mut SymbolEnv,
		method_name: &Symbol,
	) {
		let mut method_type = self.resolve_type_annotation(&method_sig.to_type_annotation(), env);
		// use the class type as the function's "this" type (or None if static)
		method_type
			.as_mut_function_sig()
			.expect("Expected method type to be a function")
			.this_type = instance_type;

		match class_env.define(
			method_name,
			SymbolKind::make_member_variable(
				method_name.clone(),
				method_type,
				false,
				instance_type.is_none(),
				method_sig.phase,
				None,
			),
			StatementIdx::Top,
		) {
			Err(type_error) => {
				self.type_error(type_error);
			}
			_ => {}
		};
	}

	fn add_module_to_env(
		&mut self,
		env: &mut SymbolEnv,
		library_name: String,
		namespace_filter: Vec<String>,
		alias: &Symbol,
		// the statement that initiated the bring, if any
		stmt: Option<&Stmt>,
	) {
		let jsii = if let Some(jsii) = self
			.jsii_imports
			.iter()
			.find(|j| j.assembly_name == library_name && j.alias.name == alias.name)
		{
			// This spec has already been pre-supplied to the typechecker, so we'll still use this to populate the symbol environment
			jsii
		} else {
			// Loading the SDK is handled different from loading any other jsii modules because with the SDK we provide an exact
			// location to locate the SDK, whereas for the other modules we need to search for them from the source directory.
			let assembly_name = if library_name == WINGSDK_ASSEMBLY_NAME {
				// in runtime, if "WINGSDK_MANIFEST_ROOT" env var is set, read it. otherwise set to "../wingsdk" for dev
				let manifest_root = std::env::var("WINGSDK_MANIFEST_ROOT").unwrap_or_else(|_| "../wingsdk".to_string());
				let assembly_name = match self.jsii_types.load_module(manifest_root.as_str()) {
					Ok(name) => name,
					Err(type_error) => {
						self.spanned_error(
							&stmt.map(|s| s.span.clone()).unwrap_or_default(),
							format!(
								"Cannot locate Wing standard library from \"{}\": {}",
								manifest_root, type_error
							),
						);
						return;
					}
				};

				assembly_name
			} else {
				let source_dir = self.source_path.parent().unwrap().to_str().unwrap();
				let assembly_name = match self.jsii_types.load_dep(library_name.as_str(), source_dir) {
					Ok(name) => name,
					Err(type_error) => {
						self.spanned_error(
							&stmt.map(|s| s.span.clone()).unwrap_or_default(),
							format!(
								"Cannot find module \"{}\" in source directory: {}",
								library_name, type_error
							),
						);
						return;
					}
				};
				assembly_name
			};

			debug!("Loaded JSII assembly {}", assembly_name);

			self.jsii_imports.push(JsiiImportSpec {
				assembly_name: assembly_name.to_string(),
				namespace_filter,
				alias: alias.clone(),
				import_statement_idx: stmt.map(|s| s.idx).unwrap_or(0),
			});

			self
				.jsii_imports
				.iter()
				.find(|j| j.assembly_name == assembly_name && j.alias.name == alias.name)
				.expect("Expected to find the just-added jsii import spec")
		};

		// check if we've already defined the given alias in the current scope
		if env
			.lookup(&jsii.alias.name.as_str().into(), Some(jsii.import_statement_idx))
			.is_some()
		{
			self.spanned_error(alias, format!("\"{}\" is already defined", alias.name));
		} else {
			let mut importer = JsiiImporter::new(&jsii, self.types, self.jsii_types);

			// If we're importing from the the wing sdk, eagerly import all the types within it
			// The wing sdk is special because it's currently the only jsii module we import with a specific target namespace
			if jsii.assembly_name == WINGSDK_ASSEMBLY_NAME {
				importer.deep_import_submodule_to_env(if jsii.namespace_filter.is_empty() {
					None
				} else {
					Some(jsii.namespace_filter.join("."))
				});
			}

			importer.import_root_types();
			importer.import_submodules_to_env(env);
		}
	}

	/// Add function arguments to the function's environment
	///
	/// #Arguments
	///
	/// * `args` - List of arguments to add, each element is a tuple of the argument symbol and whether it's
	///   reassignable arg or not. Note that the argument types are figured out from `sig`.
	/// * `sig` - The function signature (used to figure out the type of each argument).
	/// * `env` - The function's environment to prime with the args.
	///
	fn add_arguments_to_env(&mut self, args: &Vec<AstFunctionParameter>, sig: &FunctionSignature, env: &mut SymbolEnv) {
		assert!(args.len() == sig.parameters.len());
		for (arg, param) in args.iter().zip(sig.parameters.iter()) {
			match env.define(
				&arg.name,
				SymbolKind::make_free_variable(arg.name.clone(), param.typeref, arg.reassignable, env.phase),
				StatementIdx::Top,
			) {
				Err(type_error) => {
					self.type_error(type_error);
				}
				_ => {}
			};
		}
	}

	/// Hydrate `@typeparam`s in a type reference with a given type argument
	///
	/// # Arguments
	///
	/// * `env` - The environment to use for looking up the original type
	/// * `original_fqn` - The fully qualified name of the original type
	/// * `type_params` - The type argument to use for the T1, T2, .. in the original type
	///
	/// # Returns
	/// The hydrated type reference
	///
	fn hydrate_class_type_arguments(
		&mut self,
		env: &SymbolEnv,
		original_fqn: &str,
		type_params: Vec<TypeRef>,
	) -> TypeRef {
		let original_type = env.lookup_nested_str(original_fqn, None).unwrap().0.as_type().unwrap();
		let original_type_class = original_type.as_class().unwrap();
		let original_type_params = if let Some(tp) = original_type_class.type_parameters.as_ref() {
			tp
		} else {
			panic!(
				"\"{}\" does not have type parameters and does not need hydration",
				original_fqn
			);
		};

		if original_type_params.len() != type_params.len() {
			self.unspanned_error(format!(
				"Type \"{}\" has {} type parameters, but {} were provided",
				original_fqn,
				original_type_params.len(),
				type_params.len()
			));
			return self.types.error();
		}

		// map from original_type_params to type_params
		let mut types_map = HashMap::new();
		for (o, n) in original_type_params.iter().zip(type_params.iter()) {
			types_map.insert(format!("{o}"), (*o, *n));
		}

		let new_env = SymbolEnv::new(
			None,
			original_type_class.env.return_type,
			false,
			false,
			Phase::Independent,
			0,
		);
		let tt = Type::Class(Class {
			name: original_type_class.name.clone(),
			env: new_env,
			fqn: Some(original_fqn.to_string()),
			parent: original_type_class.parent,
			implements: original_type_class.implements.clone(),
			is_abstract: original_type_class.is_abstract,
			type_parameters: Some(type_params),
			phase: original_type_class.phase,
			docs: original_type_class.docs.clone(),
			std_construct_args: original_type_class.std_construct_args,
			lifts: None,
		});

		// TODO: here we add a new type regardless whether we already "hydrated" `original_type` with these `type_params`. Cache!
		let mut new_type = self.types.add_type(tt);
		let new_type_class = new_type.as_class_mut().unwrap();

		// Add symbols from original type to new type
		// Note: this is currently limited to top-level function signatures and fields
		for (name, symbol, _) in original_type_class.env.iter(true) {
			match symbol {
				SymbolKind::Variable(VariableInfo {
					name: _,
					type_: v,
					reassignable,
					phase: flight,
					kind,
					docs: _,
				}) => {
					// Replace type params in function signatures
					if let Some(sig) = v.as_function_sig() {
						let new_return_type = self.get_concrete_type_for_generic(sig.return_type, &types_map);

						let new_this_type = if let Some(this_type) = sig.this_type {
							Some(self.get_concrete_type_for_generic(this_type, &types_map))
						} else {
							None
						};

						let new_params = sig
							.parameters
							.iter()
							.map(|param| FunctionParameter {
								name: param.name.clone(),
								docs: param.docs.clone(),
								typeref: self.get_concrete_type_for_generic(param.typeref, &types_map),
							})
							.collect();

						let new_sig = FunctionSignature {
							this_type: new_this_type,
							parameters: new_params,
							return_type: new_return_type,
							phase: sig.phase,
							js_override: sig.js_override.clone(),
							docs: Docs::default(),
						};

						let sym = Symbol::global(name);
						match new_type_class.env.define(
							// TODO: Original symbol is not available. SymbolKind::Variable should probably expose it
							&sym,
							SymbolKind::make_member_variable(
								sym.clone(),
								self.types.add_type(Type::Function(new_sig)),
								*reassignable,
								matches!(kind, VariableKind::StaticMember),
								*flight,
								None,
							),
							StatementIdx::Top,
						) {
							Err(type_error) => {
								self.type_error(type_error);
							}
							_ => {}
						}
					} else {
						let new_var_type = self.get_concrete_type_for_generic(*v, &types_map);
						let var_name = Symbol::global(name);
						match new_type_class.env.define(
							// TODO: Original symbol is not available. SymbolKind::Variable should probably expose it
							&var_name,
							SymbolKind::make_member_variable(
								var_name.clone(),
								new_var_type,
								*reassignable,
								matches!(kind, VariableKind::StaticMember),
								*flight,
								None,
							),
							StatementIdx::Top,
						) {
							Err(type_error) => {
								self.type_error(type_error);
							}
							_ => {}
						}
					}
				}
				_ => {
					panic!("Unexpected symbol kind: {:?} in class env", symbol)
				}
			}
		}

		return new_type;
	}

	fn get_concrete_type_for_generic(
		&mut self,
		type_to_maybe_replace: TypeRef,
		types_map: &HashMap<String, (TypeRef, TypeRef)>,
	) -> TypeRef {
		// Lookup type to replace in the types map and return the concrete type from the maps
		if let Some(new_type_arg) = types_map
			.get(&format!("{type_to_maybe_replace}"))
			.filter(|(o, _)| type_to_maybe_replace.is_same_type_as(o))
			.map(|(_, n)| n)
		{
			return *new_type_arg;
		} else {
			// Handle generic return types
			// TODO: If a generic class has a method that returns another generic, it must be a builtin
			if let Some(c) = type_to_maybe_replace.as_class() {
				if let Some(type_parameters) = &c.type_parameters {
					// For now all our generics only have a single type parameter so use the first type parameter as our "T1"
					let t1 = type_parameters[0];
					let t1_replacement = *types_map
						.get(&format!("{t1}"))
						.filter(|(o, _)| t1.is_same_type_as(o))
						.map(|(_, n)| n)
						.expect("generic must have a type parameter");
					let fqn = format!("{}.{}", WINGSDK_STD_MODULE, c.name.name);
					return match fqn.as_str() {
						WINGSDK_MUT_ARRAY => self.types.add_type(Type::MutArray(t1_replacement)),
						WINGSDK_ARRAY => self.types.add_type(Type::Array(t1_replacement)),
						WINGSDK_MAP => self.types.add_type(Type::Map(t1_replacement)),
						WINGSDK_MUT_MAP => self.types.add_type(Type::MutMap(t1_replacement)),
						WINGSDK_SET => self.types.add_type(Type::Set(t1_replacement)),
						WINGSDK_MUT_SET => self.types.add_type(Type::MutSet(t1_replacement)),
						_ => {
							self.unspanned_error(format!("\"{}\" is not a supported generic return type", fqn));
							self.types.error()
						}
					};
				}
			}
		}
		return type_to_maybe_replace;
	}

	fn get_stdlib_symbol(&self, symbol: &Symbol) -> Option<Symbol> {
		// Need this in order to map wing types to their stdlib equivalents
		// e.g. wing::str -> stdlib::String | wing::Array -> stdlib::ImmutableArray
		match symbol.name.as_str() {
			"Json" => Some(symbol.clone()),
			"duration" => Some(Symbol {
				name: "Duration".to_string(),
				span: symbol.span.clone(),
			}),
			"str" => Some(Symbol {
				name: "String".to_string(),
				span: symbol.span.clone(),
			}),
			"num" => Some(Symbol {
				name: "Number".to_string(),
				span: symbol.span.clone(),
			}),
			"bool" => Some(Symbol {
				name: "Boolean".to_string(),
				span: symbol.span.clone(),
			}),
			_ => None,
		}
	}

	fn reference_to_udt(&mut self, reference: &Reference) -> Option<UserDefinedType> {
		// TODO: we currently don't handle parenthesized expressions correctly so something like `(MyEnum).A` or `std.(namespace.submodule).A` will return true, is this a problem?
		// https://github.com/winglang/wing/issues/1006
		let mut path = vec![];
		let mut current_reference = reference;
		loop {
			match &current_reference {
				Reference::Identifier(symbol) => {
					if let Some(stdlib_symbol) = self.get_stdlib_symbol(symbol) {
						path.push(stdlib_symbol);
						path.push(Symbol {
							name: WINGSDK_STD_MODULE.to_string(),
							span: symbol.span.clone(),
						});
					} else {
						path.push(symbol.clone());
					}
					break;
				}
				Reference::InstanceMember {
					object,
					property,
					optional_accessor: _,
				} => {
					path.push(property.clone());
					current_reference = match &object.kind {
						ExprKind::Reference(r) => r,
						_ => return None,
					}
				}
				Reference::TypeReference(type_) => {
					return Some(type_.clone());
				}
				Reference::TypeMember { typeobject, property } => {
					path.push(property.clone());
					current_reference = match &typeobject.kind {
						ExprKind::Reference(r) => r,
						_ => return None,
					}
				}
			}
		}

		let root = path.pop().unwrap();
		path.reverse();

		// combine all the spans into a single span
		let start = root.span.start;
		let end = path.last().map(|s| s.span.end).unwrap_or(root.span.end);
		let file_id = root.span.file_id.clone();

		Some(UserDefinedType {
			root,
			fields: path,
			span: WingSpan { start, end, file_id },
		})
	}

	/// Check if this expression is actually a reference to a type. The parser doesn't distinguish between a `some_expression.field` and `SomeType.field`.
	/// This function checks if the expression is a reference to a user define type and if it is it returns it. If not it returns `None`.
	fn expr_maybe_type(&mut self, expr: &Expr, env: &SymbolEnv) -> Option<UserDefinedType> {
		// TODO: we currently don't handle parenthesized expressions correctly so something like `(MyEnum).A` or `std.(namespace.submodule).A` will return true, is this a problem?
		// https://github.com/winglang/wing/issues/1006

		let base_udt = if let ExprKind::Reference(reference) = &expr.kind {
			self.reference_to_udt(reference)?
		} else {
			return None;
		};

		// rewrite "namespace.foo()" to "namespace.Util.foo()" (e.g. `util.env()`). we do this by
		// looking up the symbol path within the current environment and if it resolves to a namespace,
		// then resolve a class named "Util" within it. This will basically be equivalent to the
		// `foo.Bar.baz()` case (where `baz()`) is a static method of class `Bar`.
		if base_udt.fields.is_empty() {
			let result = env.lookup_nested_str(&base_udt.full_path_str(), Some(self.statement_idx));
			if let LookupResult::Found(symbol_kind, _) = result {
				if let SymbolKind::Namespace(_) = symbol_kind {
					let mut new_udt = base_udt.clone();
					new_udt.fields.push(Symbol {
						name: "Util".to_string(),
						span: base_udt.span.clone(),
					});

					return self
						.resolve_user_defined_type(&new_udt, env, self.statement_idx)
						.ok()
						.map(|_| new_udt);
				}
			}
		}

		self
			.resolve_user_defined_type(&base_udt, env, self.statement_idx)
			.ok()
			.map(|_| base_udt)
	}

	fn resolve_reference(&mut self, reference: &Reference, env: &SymbolEnv) -> (VariableInfo, Phase) {
		match reference {
			Reference::Identifier(symbol) => {
				let lookup_res = env.lookup_ext(symbol, Some(self.statement_idx));
				if let LookupResult::Found(var, _) = lookup_res {
					if let Some(var) = var.as_variable() {
						let phase = var.phase;
						(var, phase)
					} else {
						self.spanned_error_with_var(
							symbol,
							format!("Expected identifier \"{symbol}\" to be a variable, but it's a {var}",),
						)
					}
				} else {
					// Give a specific error message if someone tries to write "print" instead of "log"
					if symbol.name == "print" {
						self.spanned_error(symbol, "Unknown symbol \"print\", did you mean to use \"log\"?");
					} else {
						self.type_error(lookup_result_to_type_error(lookup_res, symbol));
					}
					(self.make_error_variable_info(), Phase::Independent)
				}
			}
			Reference::InstanceMember {
				object,
				property,
				optional_accessor,
			} => {
				// There's a special case where the object is actually a type and the property is either a static member or an enum variant.
				// In this case the type might even be namespaced (recursive nested reference). We need to detect this and transform this
				// reference into a type reference.
				if let Some(user_type_annotation) = self.expr_maybe_type(object, env) {
					// We can't get here twice, we can safely assume that if we're here the `object` part of the reference doesn't have and evaluated type yet.
					// Create a type reference out of this nested reference and call ourselves again
					let new_ref = Reference::TypeMember {
						typeobject: Box::new(user_type_annotation.to_expression()),
						property: property.clone(),
					};
					// Replace the reference with the new one, this is unsafe because `reference` isn't mutable and theoretically someone may
					// hold another reference to it. But our AST doesn't hold up/cross references so this is safe as long as we return right.
					let const_ptr = reference as *const Reference;
					let mut_ptr = const_ptr as *mut Reference;
					unsafe {
						// We don't use the return value but need to call replace so it'll drop the old value
						_ = std::mem::replace(&mut *mut_ptr, new_ref);
					}
					return self.resolve_reference(reference, env);
				}

				// Special case: if the object expression is a simple reference to `this` and we're inside the init function then
				// we'll consider all properties as reassignable regardless of whether they're `var`.
				let mut force_reassignable = false;
				if let ExprKind::Reference(Reference::Identifier(symb)) = &object.kind {
					if symb.name == "this" {
						if let LookupResult::Found(kind, info) = env.lookup_ext(&symb, Some(self.statement_idx)) {
							// `this` reserved symbol should always be a variable
							assert!(matches!(kind, SymbolKind::Variable(_)));
							force_reassignable = info.init;
						}
					}
				}

				let (instance_type, instance_phase) = self.type_check_exp(object, env);

				// If resolving the object's type failed, we can't resolve the property either
				if instance_type.is_unresolved() {
					return (self.make_error_variable_info(), Phase::Independent);
				}

				let mut property_variable = self.resolve_variable_from_instance_type(instance_type, property, env, object);

				// if the object is `this`, then use the property's phase instead of the object phase
				let property_phase = if property_variable.phase == Phase::Independent {
					instance_phase
				} else {
					property_variable.phase
				};

				// Check if the object is an optional type. If it is ensure the use of optional chaining.
				let object_type = self.types.get_expr_type(object);
				let object_is_option = object_type.is_option();

				if object_is_option && !optional_accessor {
					self.spanned_error(
						object,
						format!(
							"Property access on optional type \"{}\" requires optional accessor: \"?.\"",
							object_type
						),
					);
				}

				if force_reassignable {
					property_variable.reassignable = true;
				}

				// If `a?.b.c`, make sure the entire reference is optional
				if *optional_accessor {
					property_variable.type_ = self.types.make_option(property_variable.type_);
				}

				(property_variable, property_phase)
			}
			Reference::TypeReference(udt) => {
				let result = self.resolve_user_defined_type(udt, env, self.statement_idx);
				let t = match result {
					Err(e) => return self.spanned_error_with_var(udt, e.message),
					Ok(t) => t,
				};

				let phase = if let Some(c) = t.as_class() {
					c.phase
				} else {
					Phase::Independent
				};

				(
					VariableInfo {
						name: Symbol::global(udt.full_path_str()),
						type_: t,
						reassignable: false,
						phase,
						kind: VariableKind::Type,
						docs: None,
					},
					phase,
				)
			}
			Reference::TypeMember { typeobject, property } => {
				let (type_, _) = self.type_check_exp(typeobject, env);

				let ExprKind::Reference(typeref) = &typeobject.kind else {
					return self.spanned_error_with_var(typeobject, "Expecting a reference");
				};

				let Reference::TypeReference(_) = typeref else {
					return self.spanned_error_with_var(typeobject, "Expecting a reference to a type");
				};

				match *type_ {
					Type::Enum(ref e) => {
						if e.values.contains(property) {
							(
								VariableInfo {
									name: property.clone(),
									kind: VariableKind::StaticMember,
									type_,
									reassignable: false,
									phase: Phase::Independent,
									docs: None,
								},
								Phase::Independent,
							)
						} else {
							self.spanned_error_with_var(
								property,
								format!("Enum \"{}\" does not contain value \"{}\"", type_, property.name),
							)
						}
					}
					Type::Class(ref c) => match c.env.lookup(&property, None) {
						Some(SymbolKind::Variable(v)) => {
							if let VariableKind::StaticMember = v.kind {
								(v.clone(), v.phase)
							} else {
								self.spanned_error_with_var(
									property,
									format!(
										"Class \"{}\" contains a member \"{}\" but it is not static",
										type_, property.name
									),
								)
							}
						}
						_ => self.spanned_error_with_var(
							property,
							format!("No member \"{}\" in class \"{}\"", property.name, type_),
						),
					},
					_ => self.spanned_error_with_var(property, format!("\"{}\" not a valid reference", reference)),
				}
			}
		}
	}

	fn resolve_variable_from_instance_type(
		&mut self,
		instance_type: UnsafeRef<Type>,
		property: &Symbol,
		env: &SymbolEnv,
		// only used for recursion
		_object: &Expr,
	) -> VariableInfo {
		match *instance_type {
			Type::Optional(t) => self.resolve_variable_from_instance_type(t, property, env, _object),
			Type::Class(ref class) => self.get_property_from_class_like(class, property),
			Type::Interface(ref interface) => self.get_property_from_class_like(interface, property),
			Type::Anything => VariableInfo {
				name: property.clone(),
				type_: instance_type,
				reassignable: false,
				phase: env.phase,
				kind: VariableKind::InstanceMember,
				docs: None,
			},

			// Lookup wingsdk std types, hydrating generics if necessary
			Type::Array(t) => {
				let new_class = self.hydrate_class_type_arguments(env, WINGSDK_ARRAY, vec![t]);
				self.get_property_from_class_like(new_class.as_class().unwrap(), property)
			}
			Type::MutArray(t) => {
				let new_class = self.hydrate_class_type_arguments(env, WINGSDK_MUT_ARRAY, vec![t]);
				self.get_property_from_class_like(new_class.as_class().unwrap(), property)
			}
			Type::Set(t) => {
				let new_class = self.hydrate_class_type_arguments(env, WINGSDK_SET, vec![t]);
				self.get_property_from_class_like(new_class.as_class().unwrap(), property)
			}
			Type::MutSet(t) => {
				let new_class = self.hydrate_class_type_arguments(env, WINGSDK_MUT_SET, vec![t]);
				self.get_property_from_class_like(new_class.as_class().unwrap(), property)
			}
			Type::Map(t) => {
				let new_class = self.hydrate_class_type_arguments(env, WINGSDK_MAP, vec![t]);
				self.get_property_from_class_like(new_class.as_class().unwrap(), property)
			}
			Type::MutMap(t) => {
				let new_class = self.hydrate_class_type_arguments(env, WINGSDK_MUT_MAP, vec![t]);
				self.get_property_from_class_like(new_class.as_class().unwrap(), property)
			}
			Type::Json => self.get_property_from_class_like(
				env
					.lookup_nested_str(WINGSDK_JSON, None)
					.unwrap()
					.0
					.as_type()
					.unwrap()
					.as_class()
					.unwrap(),
				property,
			),
			Type::MutJson => self.get_property_from_class_like(
				env
					.lookup_nested_str(WINGSDK_MUT_JSON, None)
					.unwrap()
					.0
					.as_type()
					.unwrap()
					.as_class()
					.unwrap(),
				property,
			),
			Type::String => self.get_property_from_class_like(
				env
					.lookup_nested_str(WINGSDK_STRING, None)
					.unwrap()
					.0
					.as_type()
					.unwrap()
					.as_class()
					.unwrap(),
				property,
			),
			Type::Duration => self.get_property_from_class_like(
				env
					.lookup_nested_str(WINGSDK_DURATION, None)
					.unwrap()
					.0
					.as_type()
					.unwrap()
					.as_class()
					.unwrap(),
				property,
			),
			Type::Struct(ref s) => self.get_property_from_class_like(s, property),
			_ => {
				self
					.spanned_error_with_var(property, "Property not found".to_string())
					.0
			}
		}
	}

	/// Get's the type of an instance variable in a class
	fn get_property_from_class_like(&mut self, class: &impl ClassLike, property: &Symbol) -> VariableInfo {
		let lookup_res = class.get_env().lookup_ext(property, None);
		if let LookupResult::Found(field, _) = lookup_res {
			let var = field.as_variable().expect("Expected property to be a variable");
			if let VariableKind::StaticMember = var.kind {
				self
					.spanned_error_with_var(
						property,
						format!("Cannot access static property \"{property}\" from instance"),
					)
					.0
			} else {
				var
			}
		} else {
			self.type_error(lookup_result_to_type_error(lookup_res, property));
			self.make_error_variable_info()
		}
	}

	/// Resolves a user defined type (e.g. `Foo.Bar.Baz`) to a type reference
	/// If needed, this method can also resolve types from jsii libraries that have yet to be imported
	fn resolve_user_defined_type(
		&mut self,
		user_defined_type: &UserDefinedType,
		env: &SymbolEnv,
		statement_idx: usize,
	) -> Result<TypeRef, TypeError> {
		// Attempt to resolve the type from the current environment
		let res = resolve_user_defined_type(user_defined_type, env, statement_idx);
		if res.is_ok() {
			return res;
		}

		// If the type is not found, attempt to import it from a jsii library
		if import_udt_from_jsii(self.types, self.jsii_types, user_defined_type, &self.jsii_imports) {
			return resolve_user_defined_type(user_defined_type, env, statement_idx);
		}

		// If the type is still not found, return the original error
		res
	}

	fn extract_parent_class(
		&mut self,
		parent_expr: Option<&Expr>,
		phase: Phase,
		name: &Symbol,
		env: &mut SymbolEnv,
	) -> (Option<TypeRef>, Option<SymbolEnvRef>) {
		let Some(parent_expr) = parent_expr else  {
			if phase == Phase::Preflight {
				// if this is a preflight and we don't have a parent, then we implicitly set it to `std.Resource`
				let t = self.types.resource_base_type();
				let env = t.as_preflight_class().unwrap().env.get_ref();
				return (Some(t), Some(env));
			} else {
				return (None, None);
			}
		};

		let (parent_type, _) = self.type_check_exp(&parent_expr, env);

		// bail out if we could not resolve the parent type
		if parent_type.is_unresolved() {
			return (None, None);
		}

		// Safety: we return from the function above so parent_udt cannot be None
		let parent_udt = parent_expr.as_type_reference().unwrap();

		if &parent_udt.root == name && parent_udt.fields.is_empty() {
			self.spanned_error(parent_udt, "Class cannot extend itself".to_string());
			self.types.assign_type_to_expr(parent_expr, self.types.error(), phase);
			return (None, None);
		}

		if let Some(parent_class) = parent_type.as_class() {
			if parent_class.phase == phase {
				(Some(parent_type), Some(parent_class.env.get_ref()))
			} else {
				report_diagnostic(Diagnostic {
					message: format!(
						"Class \"{}\" is an {} class and cannot extend {} class \"{}\"",
						name, phase, parent_class.phase, parent_class.name
					),
					span: Some(parent_expr.span.clone()),
				});
				self.types.assign_type_to_expr(parent_expr, self.types.error(), phase);
				(None, None)
			}
		} else {
			report_diagnostic(Diagnostic {
				message: format!("Expected \"{}\" to be a class", parent_udt),
				span: Some(parent_expr.span.clone()),
			});
			self.types.assign_type_to_expr(parent_expr, self.types.error(), phase);
			(None, None)
		}
	}
}

fn add_parent_members_to_struct_env(
	extends_types: &Vec<TypeRef>,
	name: &Symbol,
	struct_env: &mut SymbolEnv,
) -> Result<(), TypeError> {
	// Add members of all parents to the struct's environment
	for parent_type in extends_types.iter() {
		let parent_struct = if let Some(parent_struct) = parent_type.as_struct() {
			parent_struct
		} else {
			return Err(TypeError {
				message: format!(
					"Type \"{}\" extends \"{}\" which should be a struct",
					name.name, parent_type
				),
				span: name.span.clone(),
			});
		};
		// Add each member of current parent to the struct's environment (if it wasn't already added by a previous parent)
		for (parent_member_name, parent_member, _) in parent_struct.env.iter(true) {
			let member_type = parent_member
				.as_variable()
				.expect("Expected struct member to be a variable")
				.type_;
			if let Some(existing_type) = struct_env.lookup(&parent_member_name.as_str().into(), None) {
				let existing_type = existing_type
					.as_variable()
					.expect("Expected struct member to be a variable")
					.type_;
				if !existing_type.is_same_type_as(&member_type) {
					return Err(TypeError {
						span: name.span.clone(),
						message: format!(
							"Struct \"{}\" extends \"{}\" which introduces a conflicting member \"{}\" ({} != {})",
							name, parent_type, parent_member_name, member_type, member_type
						),
					});
				}
			} else {
				let sym = Symbol {
					name: parent_member_name,
					span: name.span.clone(),
				};
				struct_env.define(
					&sym,
					SymbolKind::make_member_variable(sym.clone(), member_type, false, false, struct_env.phase, None),
					StatementIdx::Top,
				)?;
			}
		}
	}
	Ok(())
}

// TODO: dup code with `add_parent_members_to_struct_env`
fn add_parent_members_to_iface_env(
	extends_types: &Vec<TypeRef>,
	name: &Symbol,
	iface_env: &mut SymbolEnv,
) -> Result<(), TypeError> {
	// Add members of all parents to the interface's environment
	for parent_type in extends_types.iter() {
		let parent_iface = if let Some(parent_iface) = parent_type.as_interface() {
			parent_iface
		} else {
			return Err(TypeError {
				message: format!(
					"Type \"{}\" extends \"{}\" which should be an interface",
					name.name, parent_type
				),
				span: name.span.clone(),
			});
		};
		// Add each member of current parent to the interface's environment (if it wasn't already added by a previous parent)
		for (parent_member_name, parent_member, _) in parent_iface.env.iter(true) {
			let member_type = parent_member
				.as_variable()
				.expect("Expected interface member to be a variable")
				.type_;
			if let Some(existing_type) = iface_env.lookup(&parent_member_name.as_str().into(), None) {
				let existing_type = existing_type
					.as_variable()
					.expect("Expected interface member to be a variable")
					.type_;
				if !existing_type.is_same_type_as(&member_type) {
					return Err(TypeError {
						span: name.span.clone(),
						message: format!(
							"Interface \"{}\" extends \"{}\" but has a conflicting member \"{}\" ({} != {})",
							name, parent_type, parent_member_name, member_type, member_type
						),
					});
				}
			} else {
				let sym = Symbol {
					name: parent_member_name,
					span: name.span.clone(),
				};
				iface_env.define(
					&sym,
					SymbolKind::make_member_variable(sym.clone(), member_type, false, true, iface_env.phase, None),
					StatementIdx::Top,
				)?;
			}
		}
	}
	Ok(())
}

fn lookup_result_to_type_error<T>(lookup_result: LookupResult, looked_up_object: &T) -> TypeError
where
	T: Spanned + Display,
{
	let (message, span) = match lookup_result {
		LookupResult::NotFound(s) => (format!("Unknown symbol \"{s}\""), s.span()),
		LookupResult::DefinedLater => (
			format!("Symbol \"{looked_up_object}\" used before being defined"),
			looked_up_object.span(),
		),
		LookupResult::ExpectedNamespace(ns_name) => (
			format!("Expected \"{ns_name}\" in \"{looked_up_object}\" to be a namespace"),
			ns_name.span(),
		),
		LookupResult::Found(..) => panic!("Expected a lookup error, but found a successful lookup"),
	};
	TypeError { message, span }
}

/// Resolves a user defined type (e.g. `Foo.Bar.Baz`) to a type reference
pub fn resolve_user_defined_type(
	user_defined_type: &UserDefinedType,
	env: &SymbolEnv,
	statement_idx: usize,
) -> Result<TypeRef, TypeError> {
	// Resolve all types down the fields list and return the last one (which is likely to be a real type and not a namespace)
	let mut nested_name = vec![&user_defined_type.root];
	nested_name.extend(user_defined_type.fields.iter().collect_vec());

	let lookup_result = env.lookup_nested(&nested_name, Some(statement_idx));

	if let LookupResult::Found(symb_kind, _) = lookup_result {
		if let SymbolKind::Type(t) = symb_kind {
			Ok(*t)
		} else {
			let symb = nested_name.last().unwrap();
			Err(TypeError {
				message: format!("Expected \"{}\" to be a type but it's a {symb_kind}", symb.name),
				span: symb.span.clone(),
			})
		}
	} else {
		Err(lookup_result_to_type_error(lookup_result, user_defined_type))
	}
}

pub fn import_udt_from_jsii(
	wing_types: &mut Types,
	jsii_types: &mut TypeSystem,
	user_defined_type: &UserDefinedType,
	jsii_imports: &[JsiiImportSpec],
) -> bool {
	for jsii in jsii_imports {
		if jsii.alias.name == user_defined_type.root.name {
			let mut importer = JsiiImporter::new(&jsii, wing_types, jsii_types);

			let mut udt_string = if jsii.assembly_name == WINGSDK_ASSEMBLY_NAME {
				// when importing from the std lib, the "alias" is the submodule
				format!("{}.{}.", jsii.assembly_name, jsii.alias.name)
			} else {
				format!("{}.", jsii.assembly_name)
			};
			udt_string.push_str(&user_defined_type.fields.iter().map(|g| g.name.clone()).join("."));

			return importer.import_type(&FQN::from(udt_string.as_str()));
		}
	}
	false
}

/// *Hacky* If the given type is from the std namespace, add the implicit `std.` to it.
///
/// This is needed because our builtin types have no API
/// So we have to get the API from the std lib
/// But the std lib sometimes doesn't have the same names as the builtin types
///
/// https://github.com/winglang/wing/issues/1780
pub fn fully_qualify_std_type(type_: &str) -> std::string::String {
	// Additionally, this doesn't handle for generics
	let type_name = type_.to_string();
	let type_name = if let Some((prefix, _)) = type_name.split_once("<") {
		prefix
	} else {
		&type_name
	};

	let type_name = match type_name {
		"str" => "String",
		"duration" => "Duration",
		"bool" => "Boolean",
		"num" => "Number",
		_ => type_name,
	};

	match type_name {
		"Json" | "MutJson" | "MutArray" | "MutMap" | "MutSet" | "Array" | "Map" | "Set" | "String" | "Duration"
		| "Boolean" | "Number" => format!("{WINGSDK_STD_MODULE}.{type_name}"),
		_ => type_name.to_string(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn phase_subtyping() {
		// subtyping is reflexive
		assert!(Phase::Independent.is_subtype_of(&Phase::Independent));
		assert!(Phase::Preflight.is_subtype_of(&Phase::Preflight));
		assert!(Phase::Inflight.is_subtype_of(&Phase::Inflight));

		// independent is a subtype of preflight
		assert!(Phase::Independent.is_subtype_of(&Phase::Preflight));
		assert!(!Phase::Preflight.is_subtype_of(&Phase::Independent));

		// independent is a subtype of inflight
		assert!(Phase::Independent.is_subtype_of(&Phase::Inflight));
		assert!(!Phase::Inflight.is_subtype_of(&Phase::Independent));

		// preflight and inflight are not subtypes of each other
		assert!(!Phase::Preflight.is_subtype_of(&Phase::Inflight));
		assert!(!Phase::Inflight.is_subtype_of(&Phase::Preflight));
	}

	fn make_function(params: Vec<FunctionParameter>, ret: TypeRef, phase: Phase) -> Type {
		Type::Function(FunctionSignature {
			this_type: None,
			parameters: params,
			return_type: ret,
			phase,
			js_override: None,
			docs: Docs::default(),
		})
	}

	#[test]
	fn optional_subtyping() {
		let string = UnsafeRef::<Type>(&Type::String as *const Type);
		let opt_string = UnsafeRef::<Type>(&Type::Optional(string) as *const Type);

		// T is a subtype of T? since T can be used anywhere a T? is expected
		// (but not vice versa)
		assert!(string.is_subtype_of(&opt_string));
		assert!(!opt_string.is_subtype_of(&string));

		// subtyping is reflexive
		assert!(string.is_subtype_of(&string));
		assert!(opt_string.is_subtype_of(&opt_string));
	}

	#[test]
	fn function_subtyping_across_phases() {
		let void = UnsafeRef::<Type>(&Type::Void as *const Type);
		let inflight_fn = make_function(vec![], void, Phase::Inflight);
		let preflight_fn = make_function(vec![], void, Phase::Preflight);

		// functions of different phases are not subtypes of each other
		assert!(!inflight_fn.is_subtype_of(&preflight_fn));
		assert!(!preflight_fn.is_subtype_of(&inflight_fn));

		// subtyping is reflexive
		assert!(inflight_fn.is_subtype_of(&inflight_fn));
		assert!(preflight_fn.is_subtype_of(&preflight_fn));
	}

	#[test]
	fn function_subtyping_incompatible_single_param() {
		let void = UnsafeRef::<Type>(&Type::Void as *const Type);
		let num = UnsafeRef::<Type>(&Type::Number as *const Type);
		let string = UnsafeRef::<Type>(&Type::String as *const Type);
		let num_fn = make_function(
			vec![FunctionParameter {
				typeref: num,
				docs: Docs::default(),
				name: "p1".into(),
			}],
			void,
			Phase::Inflight,
		);
		let str_fn = make_function(
			vec![FunctionParameter {
				typeref: string,
				docs: Docs::default(),
				name: "p1".into(),
			}],
			void,
			Phase::Inflight,
		);

		// functions of incompatible arguments are not subtypes of each other
		assert!(!num_fn.is_subtype_of(&str_fn));
		assert!(!str_fn.is_subtype_of(&num_fn));
	}

	#[test]
	fn function_subtyping_incompatible_return_type() {
		let void = UnsafeRef::<Type>(&Type::Void as *const Type);
		let num = UnsafeRef::<Type>(&Type::Number as *const Type);
		let string = UnsafeRef::<Type>(&Type::String as *const Type);
		let returns_num = make_function(vec![], num, Phase::Inflight);
		let returns_str = make_function(vec![], string, Phase::Inflight);
		let returns_void = make_function(vec![], void, Phase::Inflight);

		// functions of incompatible return types are not subtypes of each other
		assert!(!returns_num.is_subtype_of(&returns_str));
		assert!(!returns_str.is_subtype_of(&returns_num));

		// functions with specific return types are subtypes of functions with void return type
		assert!(returns_num.is_subtype_of(&returns_void));
		assert!(returns_str.is_subtype_of(&returns_void));
	}

	#[test]
	fn function_subtyping_parameter_contravariance() {
		let void = UnsafeRef::<Type>(&Type::Void as *const Type);
		let string = UnsafeRef::<Type>(&Type::String as *const Type);
		let opt_string = UnsafeRef::<Type>(&Type::Optional(string) as *const Type);
		let str_fn = make_function(
			vec![FunctionParameter {
				typeref: string,
				docs: Docs::default(),
				name: "p1".into(),
			}],
			void,
			Phase::Inflight,
		);
		let opt_str_fn = make_function(
			vec![FunctionParameter {
				typeref: opt_string,
				docs: Docs::default(),
				name: "p1".into(),
			}],
			void,
			Phase::Inflight,
		);

		// let x = (s: string) => {};
		// let y = (s: string?) => {};
		// y is a subtype of x because a function that accepts a "string?" can be used
		// in place of a function that accepts a "string", but not vice versa
		assert!(opt_str_fn.is_subtype_of(&str_fn));
		assert!(!str_fn.is_subtype_of(&opt_str_fn));
	}
}
