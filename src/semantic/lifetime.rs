//! Lifetime checking for Zyra
//!
//! Implements Rust-inspired lifetime semantics:
//! - References must not outlive their referents
//! - Lifetime annotations express relationships between references
//! - Auto-inference for simple cases (single input reference)

use std::collections::HashMap;

/// A named lifetime (e.g., 'a, 'b)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lifetime {
    pub name: String,
    pub scope_id: usize, // Which scope this lifetime is valid in
}

impl Lifetime {
    pub fn new(name: &str, scope_id: usize) -> Self {
        Self {
            name: name.to_string(),
            scope_id,
        }
    }

    /// Static lifetime that outlives everything
    pub fn static_lifetime() -> Self {
        Self {
            name: "static".to_string(),
            scope_id: 0,
        }
    }

    /// Anonymous/inferred lifetime
    pub fn anonymous(scope_id: usize) -> Self {
        Self {
            name: format!("'_{}", scope_id),
            scope_id,
        }
    }
}

/// Lifetime constraint: 'a outlives 'b (written as 'a: 'b)
#[derive(Debug, Clone)]
pub struct LifetimeConstraint {
    pub longer: Lifetime,  // This lifetime must be >=
    pub shorter: Lifetime, // than this lifetime
}

/// Tracks lifetime information for a reference
#[derive(Debug, Clone)]
pub struct ReferenceInfo {
    pub lifetime: Lifetime,
    pub source_variable: Option<String>, // What variable this references
    pub is_mutable: bool,
    pub defined_at_line: usize,
}

/// Lifetime checker for analyzing function signatures and bodies
pub struct LifetimeChecker {
    /// Known lifetimes in current context
    lifetimes: HashMap<String, Lifetime>,

    /// Constraints between lifetimes
    // constraints: Vec<LifetimeConstraint>,

    /// References being tracked
    references: HashMap<String, ReferenceInfo>,

    /// Current scope depth
    scope_depth: usize,

    /// Errors encountered
    errors: Vec<LifetimeError>,
}

impl LifetimeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            lifetimes: HashMap::new(),
            // constraints: Vec::new(),
            references: HashMap::new(),
            scope_depth: 0,
            errors: Vec::new(),
        };

        // Always have 'static lifetime
        checker
            .lifetimes
            .insert("static".to_string(), Lifetime::static_lifetime());

        checker
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exit current scope, invalidating lifetimes that end here
    pub fn exit_scope(&mut self) {
        // Find all references with lifetimes in this scope and mark them invalid
        let current_scope = self.scope_depth;
        self.references
            .retain(|_, info| info.lifetime.scope_id < current_scope);

        // Remove lifetimes that end at this scope
        self.lifetimes.retain(|_, lt| lt.scope_id < current_scope);

        self.scope_depth -= 1;
    }

    /// Declare a named lifetime parameter for a function
    pub fn declare_lifetime(&mut self, name: &str) {
        if !name.starts_with('\'') {
            return;
        }
        let clean_name = &name[1..]; // Remove leading '
        self.lifetimes.insert(
            clean_name.to_string(),
            Lifetime::new(clean_name, self.scope_depth),
        );
    }

    /// Track a reference being created
    pub fn track_reference(
        &mut self,
        ref_name: &str,
        source_var: Option<&str>,
        lifetime_name: Option<&str>,
        is_mutable: bool,
        line: usize,
    ) {
        let lifetime = match lifetime_name {
            Some(lt_name) => {
                // Use declared lifetime or create new one
                self.lifetimes
                    .get(lt_name)
                    .cloned()
                    .unwrap_or_else(|| Lifetime::new(lt_name, self.scope_depth))
            }
            None => {
                // Infer lifetime from source or create anonymous
                Lifetime::anonymous(self.scope_depth)
            }
        };

        self.references.insert(
            ref_name.to_string(),
            ReferenceInfo {
                lifetime,
                source_variable: source_var.map(|s| s.to_string()),
                is_mutable,
                defined_at_line: line,
            },
        );
    }

    /// Check if returning a reference is valid
    pub fn check_return_lifetime(
        &mut self,
        return_ref: &str,
        expected_lifetime: Option<&str>,
        line: usize,
    ) -> Result<(), LifetimeError> {
        let ref_info =
            self.references
                .get(return_ref)
                .ok_or_else(|| LifetimeError::UnknownReference {
                    name: return_ref.to_string(),
                    at_line: line,
                })?;

        // If we have an expected lifetime, verify it matches or is compatible
        if let Some(expected) = expected_lifetime {
            if let Some(expected_lt) = self.lifetimes.get(expected) {
                // The returned reference's lifetime must be >= expected lifetime
                if ref_info.lifetime.scope_id > expected_lt.scope_id {
                    return Err(LifetimeError::ReturnOutlivesScope {
                        reference: return_ref.to_string(),
                        lifetime: ref_info.lifetime.name.clone(),
                        expected: expected.to_string(),
                        at_line: line,
                    });
                }
            }
        }

        // Check that reference doesn't outlive its source
        if let Some(source) = &ref_info.source_variable {
            // The reference must not be returned if the source goes out of scope
            if ref_info.lifetime.scope_id < self.scope_depth {
                return Err(LifetimeError::DanglingReference {
                    reference: return_ref.to_string(),
                    source: source.clone(),
                    at_line: line,
                });
            }
        }

        Ok(())
    }

    /// Apply elision rules for function signatures
    ///
    /// Elision rules (like Rust):
    /// 1. Each input reference gets its own lifetime if not specified
    /// 2. If there's exactly one input lifetime, output gets same lifetime
    /// 3. If there's &self or &mut self, output gets self's lifetime
    pub fn infer_output_lifetime(
        &self,
        input_lifetimes: &[Option<String>],
        has_self: bool,
    ) -> Option<String> {
        // Rule 3: &self or &mut self
        if has_self {
            return Some("self".to_string());
        }

        // Rule 2: Single input reference
        let explicit: Vec<_> = input_lifetimes
            .iter()
            .filter_map(|lt| lt.as_ref())
            .collect();

        if explicit.len() == 1 {
            return Some(explicit[0].clone());
        }

        // Count anonymous (inferred) lifetimes
        let inferred_count = input_lifetimes.iter().filter(|lt| lt.is_none()).count();

        if explicit.is_empty() && inferred_count == 1 {
            // Single inferred input lifetime
            return Some("'_".to_string());
        }

        // Multiple lifetimes - cannot elide
        None
    }

    /// Verify function signature lifetimes are valid
    pub fn validate_function_signature(
        &mut self,
        func_name: &str,
        lifetime_params: &[String],
        param_lifetimes: &[(String, Option<String>)], // (param_name, lifetime)
        return_lifetime: Option<&str>,
        line: usize,
    ) -> Result<(), LifetimeError> {
        // Declare all lifetime parameters
        for lt in lifetime_params {
            self.declare_lifetime(lt);
        }

        // Check that return lifetime is declared or can be inferred
        if let Some(ret_lt) = return_lifetime {
            let clean_lt = ret_lt.trim_start_matches('\'');
            if !self.lifetimes.contains_key(clean_lt) && clean_lt != "static" && clean_lt != "_" {
                // Check if it matches any input lifetime
                let input_lts: Vec<_> = param_lifetimes
                    .iter()
                    .filter_map(|(_, lt)| lt.as_ref())
                    .collect();

                if !input_lts.iter().any(|lt| lt.contains(clean_lt)) {
                    return Err(LifetimeError::UndeclaredLifetime {
                        lifetime: ret_lt.to_string(),
                        function: func_name.to_string(),
                        at_line: line,
                    });
                }
            }
        }

        Ok(())
    }

    /// Get all errors
    pub fn errors(&self) -> &[LifetimeError] {
        &self.errors
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl Default for LifetimeChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Lifetime checking errors
#[derive(Debug, Clone)]
pub enum LifetimeError {
    UnknownReference {
        name: String,
        at_line: usize,
    },
    ReturnOutlivesScope {
        reference: String,
        lifetime: String,
        expected: String,
        at_line: usize,
    },
    DanglingReference {
        reference: String,
        source: String,
        at_line: usize,
    },
    UndeclaredLifetime {
        lifetime: String,
        function: String,
        at_line: usize,
    },
    LifetimeMismatch {
        expected: String,
        found: String,
        at_line: usize,
    },
    CannotInferLifetime {
        context: String,
        at_line: usize,
    },
}

impl std::fmt::Display for LifetimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifetimeError::UnknownReference { name, at_line } => {
                write!(f, "Unknown reference '{}' at line {}", name, at_line)
            }
            LifetimeError::ReturnOutlivesScope {
                reference,
                lifetime,
                expected,
                at_line,
            } => {
                write!(
                    f,
                    "Reference '{}' with lifetime '{}' cannot be returned with expected lifetime '{}' (line {})",
                    reference, lifetime, expected, at_line
                )
            }
            LifetimeError::DanglingReference {
                reference,
                source,
                at_line,
            } => {
                write!(
                    f,
                    "Dangling reference: '{}' references '{}' which goes out of scope (line {})",
                    reference, source, at_line
                )
            }
            LifetimeError::UndeclaredLifetime {
                lifetime,
                function,
                at_line,
            } => {
                write!(
                    f,
                    "Undeclared lifetime '{}' in function '{}' (line {})",
                    lifetime, function, at_line
                )
            }
            LifetimeError::LifetimeMismatch {
                expected,
                found,
                at_line,
            } => {
                write!(
                    f,
                    "Lifetime mismatch: expected '{}', found '{}' (line {})",
                    expected, found, at_line
                )
            }
            LifetimeError::CannotInferLifetime { context, at_line } => {
                write!(
                    f,
                    "Cannot infer lifetime in {}: explicit annotation required (line {})",
                    context, at_line
                )
            }
        }
    }
}
