use base64::Engine;
use dioxus_lib::prelude::dioxus_core::DynamicNode;
use dioxus_lib::prelude::{has_context, ErrorContext, ScopeId, SuspenseContext, VNode, VirtualDom};

use super::SerializeContext;

impl super::HTMLData {
    /// Walks through the suspense boundary in a depth first order and extracts the data from the context API.
    /// We use depth first order instead of relying on the order the hooks are called in because during suspense on the server, the order that futures are run in may be non deterministic.
    pub(crate) fn extract_from_suspense_boundary(vdom: &VirtualDom, scope: ScopeId) -> Self {
        let mut data = Self::default();
        data.serialize_errors(vdom, scope);
        data.take_from_scope(vdom, scope);
        data
    }

    /// Get the errors from the suspense boundary
    fn serialize_errors(&mut self, vdom: &VirtualDom, scope: ScopeId) {
        // If there is an error boundary on the suspense boundary, grab the error from the context API
        // and throw it on the client so that it bubbles up to the nearest error boundary
        let error = vdom.in_runtime(|| {
            scope
                .consume_context::<ErrorContext>()
                .and_then(|error_context| error_context.errors().first().cloned())
        });
        self.push(&error, std::panic::Location::caller());
    }

    fn take_from_scope(&mut self, vdom: &VirtualDom, scope: ScopeId) {
        vdom.in_runtime(|| {
            scope.in_runtime(|| {
                // Grab any serializable server context from this scope
                let context: Option<SerializeContext> = has_context();
                if let Some(context) = context {
                    self.extend(&context.data.borrow());
                }
            });
        });

        // then continue to any children
        if let Some(scope) = vdom.get_scope(scope) {
            // If this is a suspense boundary, move into the children first (even if they are suspended) because that will be run first on the client
            if let Some(suspense_boundary) =
                SuspenseContext::downcast_suspense_boundary_from_scope(&vdom.runtime(), scope.id())
            {
                if let Some(node) = suspense_boundary.suspended_nodes() {
                    self.take_from_vnode(vdom, &node);
                }
            }
            if let Some(node) = scope.try_root_node() {
                self.take_from_vnode(vdom, node);
            }
        }
    }

    fn take_from_vnode(&mut self, vdom: &VirtualDom, vnode: &VNode) {
        for (dynamic_node_index, dyn_node) in vnode.dynamic_nodes.iter().enumerate() {
            match dyn_node {
                DynamicNode::Component(comp) => {
                    if let Some(scope) = comp.mounted_scope(dynamic_node_index, vnode, vdom) {
                        self.take_from_scope(vdom, scope.id());
                    }
                }
                DynamicNode::Fragment(nodes) => {
                    for node in nodes {
                        self.take_from_vnode(vdom, node);
                    }
                }
                _ => {}
            }
        }
    }

    #[cfg(feature = "server")]
    /// Encode data as base64. This is intended to be used in the server to send data to the client.
    pub(crate) fn serialized(&self) -> SerializedHydrationData {
        let mut serialized = Vec::new();
        ciborium::into_writer(&self.data, &mut serialized).unwrap();

        let data = base64::engine::general_purpose::STANDARD.encode(serialized);

        let format_js_list_of_strings = |list: &[Option<String>]| {
            let body = list
                .iter()
                .map(|s| match s {
                    Some(s) => format!(r#""{s}""#),
                    None => r#""unknown""#.to_string(),
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("[{}]", body)
        };

        SerializedHydrationData {
            data,
            #[cfg(debug_assertions)]
            debug_types: format_js_list_of_strings(&self.debug_types),
            #[cfg(debug_assertions)]
            debug_locations: format_js_list_of_strings(&self.debug_locations),
        }
    }
}

#[cfg(feature = "server")]
/// Data that was serialized on the server for hydration on the client. This includes
/// extra information about the types and sources of the serialized data in debug mode
pub(crate) struct SerializedHydrationData {
    /// The base64 encoded serialized data
    pub data: String,
    /// A list of the types of each serialized data
    #[cfg(debug_assertions)]
    pub debug_types: String,
    /// A list of the locations of each serialized data
    #[cfg(debug_assertions)]
    pub debug_locations: String,
}
