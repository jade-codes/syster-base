use super::*;

impl NormalizedUsage {
    pub(super) fn from_rowan(usage: &RowanUsage) -> Self {
        // Determine usage kind based on keyword tokens
        // Check for nested transition first, then perform action
        let kind = if usage.transition_usage().is_some() {
            NormalizedUsageKind::Transition
        } else if usage.perform_action_usage().is_some() {
            // perform action => Action kind
            NormalizedUsageKind::Action
        } else {
            match usage.usage_kind() {
                Some(RowanUsageKind::Part) => NormalizedUsageKind::Part,
                Some(RowanUsageKind::Attribute) => NormalizedUsageKind::Attribute,
                Some(RowanUsageKind::Port) => NormalizedUsageKind::Port,
                Some(RowanUsageKind::Item) => NormalizedUsageKind::Item,
                Some(RowanUsageKind::Action) => NormalizedUsageKind::Action,
                Some(RowanUsageKind::State) => NormalizedUsageKind::State,
                Some(RowanUsageKind::Constraint) => NormalizedUsageKind::Constraint,
                Some(RowanUsageKind::Requirement) => NormalizedUsageKind::Requirement,
                Some(RowanUsageKind::Calc) => NormalizedUsageKind::Calculation,
                Some(RowanUsageKind::Connection) => NormalizedUsageKind::Connection,
                Some(RowanUsageKind::Interface) => NormalizedUsageKind::Interface,
                Some(RowanUsageKind::Allocation) => NormalizedUsageKind::Allocation,
                Some(RowanUsageKind::Flow) => NormalizedUsageKind::Flow,
                Some(RowanUsageKind::Occurrence) => NormalizedUsageKind::Occurrence,
                Some(RowanUsageKind::Ref) => NormalizedUsageKind::Reference,
                // KerML mappings
                Some(RowanUsageKind::Feature) => NormalizedUsageKind::Attribute, // feature -> attribute
                Some(RowanUsageKind::Step) => NormalizedUsageKind::Action,       // step -> action
                Some(RowanUsageKind::Expr) => NormalizedUsageKind::Calculation,  // expr -> calc
                Some(RowanUsageKind::Connector) => NormalizedUsageKind::Connection, // connector -> connection
                Some(RowanUsageKind::Case) => NormalizedUsageKind::Other,
                None => NormalizedUsageKind::Part, // Default to Part for usages without keyword
            }
        };

        // Extract typing as a relationship
        let mut relationships: Vec<NormalizedRelationship> = Vec::new();

        // Check for typing on the usage itself, or on nested perform action
        let typing = usage
            .typing()
            .or_else(|| usage.perform_action_usage().and_then(|p| p.typing()));
        if let Some(typing) = typing {
            if let Some(target) = typing.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TypedBy,
                    target: RelTarget::Simple(target.to_string()),
                    range: Some(target.syntax().text_range()),
                });
            }
        }

        // Extract prefix metadata (#name) as Meta relationships
        // PREFIX_METADATA nodes are preceding siblings, not children of USAGE
        for prefix_meta in usage.prefix_metadata() {
            if let (Some(name), Some(range)) = (prefix_meta.name(), prefix_meta.name_range()) {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::Meta,
                    target: RelTarget::Simple(name),
                    range: Some(range),
                });
            }
        }

        // Extract "of Type" clause (for messages, items, etc.)
        // e.g., `message sendCmd of SensedSpeed`
        if let Some(of_type) = usage.of_type() {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::TypedBy,
                target: RelTarget::Simple(of_type.to_string()),
                range: Some(of_type.syntax().text_range()),
            });
        }

        // Extract specializations
        for spec in usage.specializations() {
            // If kind is None but target exists, it's a comma-separated continuation
            // Default to Subsets since `:> A, B, C` in usages means subsetting
            let rel_kind = match spec.kind() {
                Some(SpecializationKind::Specializes) => NormalizedRelKind::Specializes,
                Some(SpecializationKind::Subsets) => NormalizedRelKind::Subsets,
                Some(SpecializationKind::Redefines) => NormalizedRelKind::Redefines,
                Some(SpecializationKind::References) => NormalizedRelKind::References,
                Some(SpecializationKind::Conjugates) => NormalizedRelKind::Specializes,
                Some(SpecializationKind::FeatureChain) => NormalizedRelKind::FeatureChain,
                None => NormalizedRelKind::Subsets, // Comma-continuation inherits Subsets for usages
            };
            if let Some(target) = spec.target() {
                let target_str = target.to_string();
                let target_range = target.syntax().text_range();
                // Check if this is a feature chain (contains .)
                let rel_target = if target_str.contains('.') {
                    // Parse as chain, using proper ranges from segments_with_ranges
                    let segments_with_ranges = target.segments_with_ranges();
                    let parts: Vec<FeatureChainPart> = segments_with_ranges
                        .iter()
                        .map(|(name, range)| FeatureChainPart {
                            name: name.clone(),
                            range: Some(*range),
                        })
                        .collect();
                    RelTarget::Chain(FeatureChain {
                        parts,
                        range: Some(target_range),
                    })
                } else {
                    RelTarget::Simple(target_str)
                };
                relationships.push(NormalizedRelationship {
                    kind: rel_kind,
                    target: rel_target,
                    range: Some(target_range),
                });
            }
        }

        // Extract expression references from expressions in this usage
        // IMPORTANT: Only extract expressions that are NOT inside nested scopes (NAMESPACE_BODY/USAGE)
        // to avoid duplicate extraction - children will extract their own expressions
        for expr in usage.descendants::<Expression>() {
            // Skip expressions that are inside a nested NAMESPACE_BODY or nested USAGE
            // These will be extracted when processing the child symbol
            let mut is_in_nested_scope = false;
            let mut ancestor = expr.syntax().parent();
            let usage_syntax = usage.syntax();
            while let Some(ref node) = ancestor {
                // Stop when we reach our own usage node (check by text_range start position)
                // Note: end position can differ due to whitespace handling, so just check start
                if node.text_range().start() == usage_syntax.text_range().start() {
                    break;
                }
                // If we hit a NAMESPACE_BODY or any USAGE/DEFINITION before reaching our own node,
                // this expression belongs to a nested scope
                let is_boundary = matches!(
                    node.kind(),
                    crate::parser::SyntaxKind::NAMESPACE_BODY
                        | crate::parser::SyntaxKind::USAGE
                        | crate::parser::SyntaxKind::DEFINITION
                );
                if is_boundary {
                    is_in_nested_scope = true;
                    break;
                }
                ancestor = node.parent();
            }
            if is_in_nested_scope {
                continue;
            }

            // Use helper to properly extract chains with `that` resolution
            extract_expression_chains(&expr, &mut relationships);

            // Extract named constructor arguments from `new Type(argName = value)` patterns
            // These resolve as Type.argName (feature of the constructed type)
            for (type_name, arg_name, arg_range) in expr.named_constructor_args() {
                let parts = vec![
                    FeatureChainPart {
                        name: type_name,
                        range: None, // Only the arg_name should be highlighted
                    },
                    FeatureChainPart {
                        name: arg_name,
                        range: Some(arg_range),
                    },
                ];
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::Expression,
                    target: RelTarget::Chain(FeatureChain {
                        parts,
                        range: Some(arg_range), // Only highlight the argument name
                    }),
                    range: Some(arg_range),
                });
            }
        }

        // Extract from-to clause for message/flow usages (e.g., `from driver.turnVehicleOn to vehicle.trigger1`)
        if let Some(from_to) = usage.from_to_clause() {
            // Extract source chain
            if let Some(source) = from_to.source() {
                if let Some(qn) = source.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::FlowSource,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }

            // Extract target chain
            if let Some(target) = from_to.target() {
                if let Some(qn) = target.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::FlowTarget,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract direct flow endpoints (e.g., `flow X.Y to A.B` without `from` keyword)
        let (direct_source, direct_target) = usage.direct_flow_endpoints();
        if let Some(qn) = direct_source {
            let target_str = qn.to_string();
            let rel_target = make_chain_or_simple(&target_str, &qn);
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::FlowSource,
                target: rel_target,
                range: Some(qn.syntax().text_range()),
            });
        }
        if let Some(qn) = direct_target {
            let target_str = qn.to_string();
            let rel_target = make_chain_or_simple(&target_str, &qn);
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::FlowTarget,
                target: rel_target,
                range: Some(qn.syntax().text_range()),
            });
        }

        // Extract transition source/target (e.g., `transition initial then off`)
        if let Some(transition) = usage.transition_usage() {
            if let Some(source_spec) = transition.source() {
                if let Some(qn) = source_spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TransitionSource,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            if let Some(target_spec) = transition.target() {
                if let Some(qn) = target_spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TransitionTarget,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract succession source/target (e.g., `first start then run then stop`)
        if let Some(succession) = usage.succession() {
            let items: Vec<_> = succession.items().collect();
            if let Some(first) = items.first() {
                if let Some(qn) = first.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::SuccessionSource,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            // All subsequent items are targets
            for item in items.iter().skip(1) {
                if let Some(qn) = item.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::SuccessionTarget,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract perform action (e.g., `perform engineStart` or `perform takePicture.focus`)
        if let Some(perform) = usage.perform_action_usage() {
            if let Some(spec) = perform.performed() {
                if let Some(qn) = spec.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Performs,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }

            // Also extract redefines/subsets from the perform action
            // e.g., `perform X redefines foo` should capture the redefines relationship
            for spec in perform.specializations().skip(1) {
                // Skip the first one (performed action), get the rest
                let rel_kind = match spec.kind() {
                    Some(SpecializationKind::Redefines) => NormalizedRelKind::Redefines,
                    Some(SpecializationKind::Subsets) => NormalizedRelKind::Subsets,
                    Some(SpecializationKind::Specializes) => NormalizedRelKind::Specializes,
                    Some(SpecializationKind::References) => NormalizedRelKind::References,
                    _ => continue,
                };
                if let Some(qn) = spec.target() {
                    let target_str = qn.to_string();
                    relationships.push(NormalizedRelationship {
                        kind: rel_kind,
                        target: make_chain_or_simple(&target_str, &qn),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract satisfy/verify (e.g., `satisfy speedRequirement`, `verify SafetyReq by TestCase`)
        if let Some(req_ver) = usage.requirement_verification() {
            let kind = if req_ver.is_satisfy() {
                NormalizedRelKind::Satisfies
            } else {
                NormalizedRelKind::Verifies
            };

            if let Some(qn) = req_ver.requirement() {
                relationships.push(NormalizedRelationship {
                    kind,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }

            // Also extract typing if present (e.g., `sv:SafetyViewpoint` - SafetyViewpoint is the type)
            // This allows hover on the type name to work
            if let Some(typing) = req_ver.typing() {
                if let Some(target) = typing.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TypedBy,
                        target: RelTarget::Simple(target.to_string()),
                        range: Some(target.syntax().text_range()),
                    });
                }
            }

            // Extract 'by' target (e.g., `vehicle_b` in `satisfy R by vehicle_b`)
            // This is the subject/verifier being bound
            if let Some(by_target) = req_ver.by_target() {
                let target_str = by_target.to_string();
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::References, // Use References for the by-target
                    target: make_chain_or_simple(&target_str, &by_target),
                    range: Some(by_target.syntax().text_range()),
                });
            }
        }

        // Extract connect endpoints (e.g., `connect engine.output to wheel.input`)
        // First check for nested connect_usage, then direct connector_part on the usage
        // We'll collect endpoint usages here and add them to children later
        let connector_part = if let Some(connect) = usage.connect_usage() {
            connect.connector_part()
        } else {
            // For connection usages with inline connect: `connection x connect (...)`
            usage.connector_part()
        };

        // Collect endpoint usages to add to children later (after children is initialized)
        let mut endpoint_usages: Vec<NormalizedElement> = Vec::new();
        if let Some(part) = connector_part {
            // Extract all connector ends, not just source/target
            for end in part.ends() {
                // Extract endpoint name if present (LHS of ::>)
                // This is the redefinition name like `cause1` in `cause1 ::> a`
                // Create it as a child usage so it becomes a symbol that can be hovered
                if let Some(endpoint_qn) = end.endpoint_name() {
                    let endpoint_name = endpoint_qn.to_string();

                    // The target reference (RHS of ::>) becomes a References relationship
                    let mut endpoint_rels = Vec::new();
                    if let Some(target_qn) = end.target() {
                        let target_str = target_qn.to_string();
                        let rel_target = make_chain_or_simple(&target_str, &target_qn);
                        endpoint_rels.push(NormalizedRelationship {
                            kind: NormalizedRelKind::References,
                            target: rel_target,
                            range: Some(target_qn.syntax().text_range()),
                        });
                    }

                    // Create the endpoint as a child usage
                    endpoint_usages.push(NormalizedElement::Usage(NormalizedUsage {
                        name: Some(endpoint_name),
                        short_name: None,
                        kind: NormalizedUsageKind::End, // Connection endpoint
                        relationships: endpoint_rels,
                        range: Some(endpoint_qn.syntax().text_range()),
                        name_range: Some(endpoint_qn.syntax().text_range()),
                        short_name_range: None,
                        doc: None,
                        children: Vec::new(),
                        is_abstract: false,
                        is_variation: false,
                        is_readonly: false,
                        is_derived: false,
                        is_parallel: false,
                        is_individual: false,
                        is_end: true, // This is an endpoint
                        is_default: false,
                        is_ordered: false,
                        is_nonunique: false,
                        is_portion: false,
                        direction: None,
                        multiplicity: None,
                        value: None,
                    }));
                } else if let Some(qn) = end.target() {
                    // No endpoint name, just a direct reference (e.g., `a.port to b.port`)
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectTarget,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract bind endpoints (e.g., `bind port1 = port2`)
        if let Some(bind) = usage.binding_connector() {
            if let Some(qn) = bind.source() {
                let target_str = qn.to_string();
                let rel_target = make_chain_or_simple(&target_str, &qn);
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::BindSource,
                    target: rel_target,
                    range: Some(qn.syntax().text_range()),
                });
            }
            if let Some(qn) = bind.target() {
                let target_str = qn.to_string();
                let rel_target = make_chain_or_simple(&target_str, &qn);
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::BindTarget,
                    target: rel_target,
                    range: Some(qn.syntax().text_range()),
                });
            }
        }

        // Extract exhibit (e.g., `exhibit runningState`)
        if usage.is_exhibit() {
            // Look for qualified name that's the exhibited element
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Exhibits,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract include (e.g., `include useCase`)
        if usage.is_include() {
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Includes,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract assert (e.g., `assert constraint`)
        if usage.is_assert() && usage.requirement_verification().is_none() {
            // assert without satisfy/verify - standalone constraint assertion
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Asserts,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract assume (e.g., `assume precondition`)
        if usage.is_assume() {
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Assumes,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract require (e.g., `require constraint`)
        if usage.is_require() {
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Requires,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract allocate (e.g., `allocate function to component`)
        // Allocations use qualified names directly - may be feature chains like `a.b.c`
        if usage.is_allocate() {
            let qnames: Vec<_> = usage
                .syntax()
                .children()
                .filter_map(crate::parser::QualifiedName::cast)
                .collect();
            if !qnames.is_empty() {
                let source_str = qnames[0].to_string();
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::AllocateSource,
                    target: make_chain_or_simple(&source_str, &qnames[0]),
                    range: Some(qnames[0].syntax().text_range()),
                });
            }
            if qnames.len() >= 2 {
                let target_str = qnames[1].to_string();
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::AllocateTo,
                    target: make_chain_or_simple(&target_str, &qnames[1]),
                    range: Some(qnames[1].syntax().text_range()),
                });
            }
        }

        // Extract children from body
        // For perform actions, the body is inside the PERFORM_ACTION_USAGE
        // For satisfy/verify blocks, children are in CONSTRAINT_BODY not NAMESPACE_BODY
        let mut children: Vec<NormalizedElement> =
            if let Some(perform) = usage.perform_action_usage() {
                perform
                    .body()
                    .map(|b| {
                        b.members()
                            .map(|m| NormalizedElement::from_rowan(&m))
                            .collect()
                    })
                    .unwrap_or_default()
            } else if let Some(constraint_body) = usage.constraint_body() {
                // Satisfy/verify blocks use CONSTRAINT_BODY for their children
                constraint_body
                    .members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            } else {
                usage
                    .body()
                    .map(|b| {
                        b.members()
                            .map(|m| NormalizedElement::from_rowan(&m))
                            .collect()
                    })
                    .unwrap_or_default()
            };

        // For transitions, also add accept payload as a child if present
        if let Some(trans) = usage.transition_usage() {
            if let Some(accept_name) = trans.accept_payload_name() {
                let payload_text = accept_name.text();
                let payload_short = accept_name.short_name().and_then(|sn| sn.text());
                let payload_range = Some(accept_name.syntax().text_range());
                let payload_short_range =
                    accept_name.short_name().map(|sn| sn.syntax().text_range());

                // Get accept typing if present (e.g., `accept sig : Signal`)
                let mut payload_rels = Vec::new();
                if let Some(typing) = trans.accept_typing() {
                    if let Some(target) = typing.target() {
                        payload_rels.push(NormalizedRelationship {
                            kind: NormalizedRelKind::TypedBy,
                            target: RelTarget::Simple(target.to_string()),
                            range: Some(target.syntax().text_range()),
                        });
                    }
                }

                // Get accept via if present (e.g., `accept sig via port`)
                if let Some(via_target) = trans.accept_via() {
                    let target_str = via_target.to_string();
                    payload_rels.push(NormalizedRelationship {
                        kind: NormalizedRelKind::AcceptVia,
                        target: make_chain_or_simple(&target_str, &via_target),
                        range: Some(via_target.syntax().text_range()),
                    });
                }

                children.push(NormalizedElement::Usage(NormalizedUsage {
                    name: payload_text,
                    short_name: payload_short,
                    kind: NormalizedUsageKind::Accept,
                    range: payload_range,
                    name_range: payload_range,
                    short_name_range: payload_short_range,
                    doc: None,
                    relationships: payload_rels,
                    children: Vec::new(),
                    is_abstract: false,
                    is_variation: false,
                    is_readonly: false,
                    is_derived: false,
                    is_parallel: false,
                    is_individual: false,
                    is_end: false,
                    is_default: false,
                    is_ordered: false,
                    is_nonunique: false,
                    is_portion: false,
                    direction: None,
                    multiplicity: None,
                    value: None,
                }));
            }
        }

        // Get name from nested transition or perform action if present, otherwise from usage itself
        let (name, short_name, name_range, short_name_range) =
            if let Some(trans) = usage.transition_usage() {
                let trans_name = trans.name();
                (
                    trans_name.as_ref().and_then(|n| n.text()),
                    trans_name
                        .as_ref()
                        .and_then(|n| n.short_name())
                        .and_then(|sn| sn.text()),
                    trans_name.as_ref().map(|n| n.syntax().text_range()),
                    trans_name
                        .and_then(|n| n.short_name())
                        .map(|sn| sn.syntax().text_range()),
                )
            } else if let Some(perform) = usage.perform_action_usage() {
                // For perform statements, the name can come from:
                // 1. Explicit NAME node: `perform action startVehicle : StartAction`
                // 2. Performed action reference when typing present: `perform providePower : ProvidePower`
                //    In this case, `providePower` is both the name and the performed action reference
                let perform_name = perform.name();
                if let Some(ref pn) = perform_name {
                    // Explicit name exists
                    (
                        pn.text(),
                        pn.short_name().and_then(|sn| sn.text()),
                        Some(pn.syntax().text_range()),
                        pn.short_name().map(|sn| sn.syntax().text_range()),
                    )
                } else if perform.typing().is_some() {
                    // No explicit name, but has typing - use the performed action name as the usage name
                    // This handles: `perform providePower : ProvidePower;`
                    // The `providePower` is the name, `ProvidePower` is the type
                    if let Some(performed) = perform.performed() {
                        if let Some(target) = performed.target() {
                            let name_str = target.to_string();
                            // Only use simple names as the perform name (not qualified like ActionTree::foo)
                            if !name_str.contains("::") && !name_str.contains('.') {
                                (
                                    Some(name_str),
                                    None,
                                    Some(target.syntax().text_range()),
                                    None,
                                )
                            } else {
                                (None, None, None, None)
                            }
                        } else {
                            (None, None, None, None)
                        }
                    } else {
                        (None, None, None, None)
                    }
                } else {
                    (None, None, None, None)
                }
            } else {
                // Check for multiple names (e.g., `end self2 [1] feature sameThing: Anything`)
                // In this case, the first name is the identification/short name,
                // and the second name (after 'feature' keyword) is the actual feature name.
                let all_names = usage.names();
                if all_names.len() >= 2 {
                    // Multiple names: first is identification, second is feature name
                    // This handles KerML patterns like: end self2 [1] feature sameThing: ...
                    let identification = &all_names[0];
                    let feature_name = &all_names[1];
                    (
                        feature_name.text(),
                        identification.text(), // The first name becomes the short_name
                        feature_name.syntax().text_range().into(),
                        Some(identification.syntax().text_range()),
                    )
                } else {
                    let usage_name = usage.name();
                    if usage_name.is_some() {
                        // Explicit name present
                        (
                            usage_name.as_ref().and_then(|n| n.text()),
                            usage_name
                                .as_ref()
                                .and_then(|n| n.short_name())
                                .and_then(|sn| sn.text()),
                            usage_name.as_ref().map(|n| n.syntax().text_range()),
                            usage_name
                                .and_then(|n| n.short_name())
                                .map(|sn| sn.syntax().text_range()),
                        )
                    } else {
                        // No explicit name - check for shorthand redefines (`:>> name`)
                        // In SysML, `:>> name` is equivalent to naming the element with `name`
                        // and adding a redefines relationship to the parent's `name` feature.
                        // This ONLY applies to the `:>>` operator, NOT the `redefines` keyword.
                        let redefines_name = usage.specializations().find_map(|spec| {
                            // Only consider shorthand redefines (:>>), not keyword (redefines)
                            if spec.is_shorthand_redefines() {
                                spec.target().and_then(|t| {
                                    let target_str = t.to_string();
                                    // Only use simple names (not qualified names like A::B)
                                    if !target_str.contains("::") && !target_str.contains('.') {
                                        Some((target_str, t.syntax().text_range()))
                                    } else {
                                        None
                                    }
                                })
                            } else {
                                None
                            }
                        });

                        if let Some((name, range)) = redefines_name {
                            (Some(name), None, Some(range), None)
                        } else {
                            (None, None, None, None)
                        }
                    }
                }
            };

        // Add implicit typing for transition usages without explicit typing
        // In SysML, transitions are implicitly typed by TransitionAction
        // Note: We use None for range since there's no actual source text to highlight
        if kind == NormalizedUsageKind::Transition {
            let has_typing = relationships
                .iter()
                .any(|r| matches!(r.kind, NormalizedRelKind::TypedBy));
            if !has_typing {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TypedBy,
                    target: RelTarget::Simple("Actions::TransitionAction".to_string()),
                    range: None, // Implicit - no source text to highlight
                });
            }
        }

        // Add endpoint usages (collected earlier, for named endpoints like `cause1` in `cause1 ::> a`)
        children.extend(endpoint_usages);

        Self {
            name,
            short_name,
            kind,
            range: Some(usage.syntax().text_range()),
            name_range,
            short_name_range,
            doc: parser::extract_doc_comment(usage.syntax()),
            relationships,
            children,
            is_abstract: usage.is_abstract(),
            is_variation: usage.is_variation(),
            is_readonly: usage.is_readonly(),
            is_derived: usage.is_derived(),
            is_parallel: usage.is_parallel(),
            is_individual: usage.is_individual(),
            is_end: usage.is_end(),
            is_default: usage.is_default(),
            is_ordered: usage.is_ordered(),
            is_nonunique: usage.is_nonunique(),
            is_portion: usage.is_portion(),
            direction: usage.direction(),
            multiplicity: usage
                .multiplicity()
                .map(|(l, u)| Multiplicity { lower: l, upper: u }),
            value: usage
                .value_expression()
                .map(|expr| extract_value_expression(&expr)),
        }
    }

    /// Create a NormalizedUsage from a standalone BindingConnector
    pub(super) fn from_bind(bind: &parser::BindingConnector) -> Self {
        let mut relationships = Vec::new();

        if let Some(qn) = bind.source() {
            let target_str = qn.to_string();
            let rel_target = make_chain_or_simple(&target_str, &qn);
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::BindSource,
                target: rel_target,
                range: Some(qn.syntax().text_range()),
            });
        }
        if let Some(qn) = bind.target() {
            let target_str = qn.to_string();
            let rel_target = make_chain_or_simple(&target_str, &qn);
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::BindTarget,
                target: rel_target,
                range: Some(qn.syntax().text_range()),
            });
        }

        Self {
            name: None, // Bind statements are anonymous
            short_name: None,
            kind: NormalizedUsageKind::Connection, // Bind is a kind of connection
            range: Some(bind.syntax().text_range()),
            name_range: None,
            short_name_range: None,
            doc: None,
            relationships,
            children: Vec::new(),
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    /// Create a NormalizedUsage from a standalone Succession
    pub(super) fn from_succession(succ: &parser::Succession) -> Self {
        let mut relationships = Vec::new();
        let mut children = Vec::new();

        // Handle succession items (wrapped in SUCCESSION_ITEM)
        let items: Vec<_> = succ.items().collect();
        if !items.is_empty() {
            // First item is the source
            if let Some(qn) = items[0].target() {
                let target_str = qn.to_string();
                let rel_target = make_chain_or_simple(&target_str, &qn);
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::SuccessionSource,
                    target: rel_target,
                    range: Some(qn.syntax().text_range()),
                });
            } else if let Some(usage) = items[0].usage() {
                // Inline usage definition - add as child
                children.push(NormalizedElement::Usage(NormalizedUsage::from_rowan(
                    &usage,
                )));
            }
        }

        // Remaining wrapped items are targets
        for item in items.iter().skip(1) {
            if let Some(qn) = item.target() {
                let target_str = qn.to_string();
                let rel_target = make_chain_or_simple(&target_str, &qn);
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::SuccessionTarget,
                    target: rel_target,
                    range: Some(qn.syntax().text_range()),
                });
            } else if let Some(usage) = item.usage() {
                // Inline usage definition (like `then action a { ... }`)
                children.push(NormalizedElement::Usage(NormalizedUsage::from_rowan(
                    &usage,
                )));
            }
        }

        // Handle inline usages directly inside succession (not wrapped in SUCCESSION_ITEM)
        // These come from `then action a { ... }` style successions
        for usage in succ.inline_usages() {
            children.push(NormalizedElement::Usage(NormalizedUsage::from_rowan(
                &usage,
            )));
        }

        // Handle accept actions inside succession (e.g., `then action trigger accept ignitionCmd`)
        for accept in succ
            .syntax()
            .children()
            .filter_map(parser::AcceptActionUsage::cast)
        {
            children.push(NormalizedElement::Usage(
                NormalizedUsage::from_accept_action(&accept),
            ));
        }

        // Handle send actions inside succession (e.g., `then action sender send msg`)
        for send in succ
            .syntax()
            .children()
            .filter_map(parser::SendActionUsage::cast)
        {
            children.push(NormalizedElement::Usage(NormalizedUsage::from_send_action(
                &send,
            )));
        }

        // Compute a tighter range that excludes trailing whitespace
        // The syntax range includes trailing whitespace, which causes incorrect spans
        let range = {
            let full_range = succ.syntax().text_range();
            // Find the last non-whitespace token to get a tighter end position
            let mut last_significant_end = full_range.start();
            for token in succ.syntax().descendants_with_tokens() {
                if let Some(tok) = token.as_token() {
                    if tok.kind() != crate::parser::SyntaxKind::WHITESPACE
                        && tok.kind() != crate::parser::SyntaxKind::LINE_COMMENT
                        && tok.kind() != crate::parser::SyntaxKind::BLOCK_COMMENT
                    {
                        let tok_end = tok.text_range().end();
                        if tok_end > last_significant_end {
                            last_significant_end = tok_end;
                        }
                    }
                }
            }
            rowan::TextRange::new(full_range.start(), last_significant_end)
        };

        Self {
            name: None, // Succession statements are anonymous
            short_name: None,
            kind: NormalizedUsageKind::Other, // Succession as "other"
            range: Some(range),
            name_range: None,
            short_name_range: None,
            doc: None,
            relationships,
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    /// Create a NormalizedUsage from a standalone TransitionUsage
    pub(super) fn from_transition(trans: &parser::TransitionUsage) -> Self {
        let mut relationships = Vec::new();

        // Extract transition source (first specialization)
        if let Some(source_spec) = trans.source() {
            if let Some(qn) = source_spec.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TransitionSource,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }

        // Extract transition target (second specialization)
        if let Some(target_spec) = trans.target() {
            if let Some(qn) = target_spec.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TransitionTarget,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }

        // For accept transitions: look for typing and qualified names directly
        // accept sig : Signal then running; has TYPING(Signal) and QUALIFIED_NAME(running) as children
        use crate::parser::SyntaxKind;
        for child in trans.syntax().children() {
            if let Some(typing) = parser::Typing::cast(child.clone()) {
                if let Some(target) = typing.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TypedBy,
                        target: RelTarget::Simple(target.to_string()),
                        range: Some(target.syntax().text_range()),
                    });
                }
            }
            // Look for QUALIFIED_NAME after THEN_KW as transition target
            if child.kind() == SyntaxKind::QUALIFIED_NAME {
                if let Some(qn) = parser::QualifiedName::cast(child.clone()) {
                    // Check if this is after 'then' (a target), not part of accept
                    // We can't easily check position here, so add as potential target
                    // Skip if it's already been added as a typing target
                    let target_str = qn.to_string();
                    let already_exists = relationships
                        .iter()
                        .any(|r| matches!(&r.target, RelTarget::Simple(t) if t == &target_str));
                    if !already_exists {
                        relationships.push(NormalizedRelationship {
                            kind: NormalizedRelKind::TransitionTarget,
                            target: RelTarget::Simple(target_str),
                            range: Some(qn.syntax().text_range()),
                        });
                    }
                }
            }
        }

        // Extract name if present (e.g., transition myTransition first source then target)
        let name = trans.name().and_then(|n| n.text());
        let short_name = trans
            .name()
            .and_then(|n| n.short_name())
            .and_then(|sn| sn.text());
        let name_range = trans.name().map(|n| n.syntax().text_range());
        let short_name_range = trans
            .name()
            .and_then(|n| n.short_name())
            .map(|sn| sn.syntax().text_range());

        // Extract accept payload as a child symbol
        let mut children = Vec::new();
        if let Some(accept_name) = trans.accept_payload_name() {
            let payload_text = accept_name.text();
            let payload_short = accept_name.short_name().and_then(|sn| sn.text());
            let payload_range = Some(accept_name.syntax().text_range());
            let payload_short_range = accept_name.short_name().map(|sn| sn.syntax().text_range());

            // Get accept typing if present (e.g., `accept sig : Signal`)
            let mut payload_rels = Vec::new();
            if let Some(typing) = trans.accept_typing() {
                if let Some(target) = typing.target() {
                    payload_rels.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TypedBy,
                        target: RelTarget::Simple(target.to_string()),
                        range: Some(target.syntax().text_range()),
                    });
                }
            }

            children.push(NormalizedElement::Usage(NormalizedUsage {
                name: payload_text,
                short_name: payload_short,
                kind: NormalizedUsageKind::Accept,
                range: payload_range,
                name_range: payload_range,
                short_name_range: payload_short_range,
                doc: None,
                relationships: payload_rels,
                children: Vec::new(),
                is_abstract: false,
                is_variation: false,
                is_readonly: false,
                is_derived: false,
                is_parallel: false,
                is_individual: false,
                is_end: false,
                is_default: false,
                is_ordered: false,
                is_nonunique: false,
                is_portion: false,
                direction: None,
                multiplicity: None,
                value: None,
            }));
        }

        // Add implicit typing for transition usages that don't have explicit types
        // In SysML, transitions are implicitly typed by TransitionAction
        // Note: We use None for range since there's no actual source text to highlight
        let has_typing = relationships
            .iter()
            .any(|r| matches!(r.kind, NormalizedRelKind::TypedBy));
        if !has_typing {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::TypedBy,
                target: RelTarget::Simple("Actions::TransitionAction".to_string()),
                range: None, // Implicit - no source text to highlight
            });
        }

        Self {
            name,
            short_name,
            kind: NormalizedUsageKind::Transition,
            range: Some(trans.syntax().text_range()),
            name_range,
            short_name_range,
            doc: None,
            relationships,
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    /// Create a NormalizedUsage from a KerML Connector
    pub(super) fn from_connector(conn: &parser::Connector) -> Self {
        let mut relationships = Vec::new();

        // Extract connector ends if present
        if let Some(conn_part) = conn.connector_part() {
            let ends: Vec<_> = conn_part.ends().collect();
            if let Some(first) = ends.first() {
                if let Some(qn) = first.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectSource,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            for end in ends.iter().skip(1) {
                if let Some(qn) = end.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectTarget,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract children from body
        let children = conn
            .body()
            .map(|b| {
                b.members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            name: conn.name().and_then(|n| n.text()),
            short_name: conn
                .name()
                .and_then(|n| n.short_name())
                .and_then(|sn| sn.text()),
            kind: NormalizedUsageKind::Connection, // KerML connector
            range: Some(conn.syntax().text_range()),
            name_range: conn.name().map(|n| n.syntax().text_range()),
            short_name_range: conn
                .name()
                .and_then(|n| n.short_name())
                .map(|sn| sn.syntax().text_range()),
            doc: None,
            relationships,
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    /// Create a NormalizedUsage from a SysML ConnectUsage
    pub(super) fn from_connect_usage(conn: &parser::ConnectUsage) -> Self {
        let mut relationships = Vec::new();

        // Extract connector ends if present
        if let Some(conn_part) = conn.connector_part() {
            if let Some(source) = conn_part.source() {
                if let Some(qn) = source.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectSource,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            if let Some(target) = conn_part.target() {
                if let Some(qn) = target.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectTarget,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract name from NAME child if present
        let name = conn
            .syntax()
            .children()
            .find_map(parser::Name::cast)
            .and_then(|n| n.text());
        let name_range = conn
            .syntax()
            .children()
            .find_map(parser::Name::cast)
            .map(|n| n.syntax().text_range());

        Self {
            name,
            short_name: None,
            kind: NormalizedUsageKind::Connection,
            range: Some(conn.syntax().text_range()),
            name_range,
            short_name_range: None,
            doc: None,
            relationships,
            children: Vec::new(), // ConnectUsage typically has no body children
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    pub(super) fn from_send_action(send: &parser::SendActionUsage) -> Self {
        // Extract children from the send action's body
        let children: Vec<NormalizedElement> = send
            .syntax()
            .children()
            .find_map(parser::NamespaceBody::cast)
            .map(|body| {
                body.members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            })
            .unwrap_or_default();

        // Extract name - first try inside the SEND_ACTION_USAGE node,
        // then check preceding sibling (for patterns like `action sendStatus send ...`)
        let mut name = send
            .syntax()
            .children()
            .find_map(parser::Name::cast)
            .and_then(|n| n.text());
        let mut name_range = send
            .syntax()
            .children()
            .find_map(parser::Name::cast)
            .map(|n| n.syntax().text_range());

        // If no name inside, check preceding sibling (grammar puts name before SEND_ACTION_USAGE)
        if name.is_none() {
            if let Some(prev_sibling) = send.syntax().prev_sibling() {
                if let Some(name_node) = parser::Name::cast(prev_sibling) {
                    name = name_node.text();
                    name_range = Some(name_node.syntax().text_range());
                }
            }
        }

        Self {
            name,
            short_name: None,
            kind: NormalizedUsageKind::Action,
            range: Some(send.syntax().text_range()),
            name_range,
            short_name_range: None,
            doc: None,
            relationships: Vec::new(),
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    pub(super) fn from_accept_action(accept: &parser::AcceptActionUsage) -> Self {
        let mut relationships = Vec::new();
        let mut children = Vec::new();

        // Extract typing (: Type) for the accepted signal
        let payload_type = accept
            .syntax()
            .children()
            .find_map(parser::Typing::cast)
            .and_then(|t| t.target().map(|qn| qn.to_string()));

        if let Some(typing) = accept.syntax().children().find_map(parser::Typing::cast) {
            if let Some(target) = typing.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TypedBy,
                    target: RelTarget::Simple(target.to_string()),
                    range: Some(target.syntax().text_range()),
                });
            }
        }

        // Extract 'via' port target (e.g., `ignitionCmdPort` in `accept sig via ignitionCmdPort`)
        if let Some(via_target) = accept.via() {
            let target_str = via_target.to_string();
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::AcceptVia,
                target: make_chain_or_simple(&target_str, &via_target),
                range: Some(via_target.syntax().text_range()),
            });
        }

        // Extract name - two patterns to handle:
        // 1. `action trigger1 accept sig:Signal via port` - name (trigger1) is preceding sibling
        // 2. `accept ignitionCmd : IgnitionCmd via port` - name (ignitionCmd) is inside node
        // Check sibling FIRST, fallback to inside for standalone accept
        let mut name = None;
        let mut name_range = None;
        let mut payload_name = None;
        let mut payload_name_range = None;

        // First check preceding sibling (for `action <name> accept ...` pattern)
        if let Some(prev_sibling) = accept.syntax().prev_sibling() {
            if let Some(name_node) = parser::Name::cast(prev_sibling) {
                name = name_node.text();
                name_range = Some(name_node.syntax().text_range());

                // For pattern 1, the NAME inside is the payload name, not the action name
                if let Some(inner_name) = accept.syntax().children().find_map(parser::Name::cast) {
                    payload_name = inner_name.text();
                    payload_name_range = Some(inner_name.syntax().text_range());
                }
            }
        }

        // If no sibling name, check inside node (for standalone `accept <name> ...` pattern)
        if name.is_none() {
            if let Some(name_node) = accept.syntax().children().find_map(parser::Name::cast) {
                name = name_node.text();
                name_range = Some(name_node.syntax().text_range());
            }
        }

        // For `action trigger1 accept ignitionCmd : IgnitionCmd` pattern,
        // create the payload as a child item so trigger1.ignitionCmd resolves
        if let Some(pname) = payload_name {
            let mut payload_rels = Vec::new();
            if let Some(ptype) = &payload_type {
                payload_rels.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TypedBy,
                    target: RelTarget::Simple(ptype.clone()),
                    range: None,
                });
            }
            children.push(NormalizedElement::Usage(NormalizedUsage {
                name: Some(pname),
                short_name: None,
                kind: NormalizedUsageKind::Item,
                range: payload_name_range,
                name_range: payload_name_range,
                short_name_range: None,
                doc: None,
                relationships: payload_rels,
                children: Vec::new(),
                is_abstract: false,
                is_variation: false,
                is_readonly: false,
                is_derived: false,
                is_parallel: false,
                is_individual: false,
                is_end: false,
                is_default: false,
                is_ordered: false,
                is_nonunique: false,
                is_portion: false,
                direction: None,
                multiplicity: None,
                value: None,
            }));
        }

        // Extract additional children from the accept action's body (if any)
        if let Some(body) = accept
            .syntax()
            .children()
            .find_map(parser::NamespaceBody::cast)
        {
            for member in body.members() {
                children.push(NormalizedElement::from_rowan(&member));
            }
        }

        Self {
            name,
            short_name: None,
            kind: NormalizedUsageKind::Action,
            range: Some(accept.syntax().text_range()),
            name_range,
            short_name_range: None,
            doc: None,
            relationships,
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    /// Create a NormalizedUsage from a StateSubaction (entry/do/exit)
    pub(super) fn from_state_subaction(subaction: &parser::StateSubaction) -> Self {
        let mut relationships = Vec::new();

        // Extract typing if present (e.g., entry action myAction : ActionType)
        if let Some(typing) = subaction.syntax().children().find_map(parser::Typing::cast) {
            if let Some(target) = typing.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TypedBy,
                    target: RelTarget::Simple(target.to_string()),
                    range: Some(target.syntax().text_range()),
                });
            }
        }

        // Extract name if present
        let name = subaction.name().and_then(|n| n.text());
        let short_name = subaction
            .name()
            .and_then(|n| n.short_name())
            .and_then(|sn| sn.text());
        let name_range = subaction.name().map(|n| n.syntax().text_range());
        let short_name_range = subaction
            .name()
            .and_then(|n| n.short_name())
            .map(|sn| sn.syntax().text_range());

        // Extract children from the body (if any)
        let children: Vec<NormalizedElement> = subaction
            .body()
            .map(|body| {
                body.members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            name,
            short_name,
            kind: NormalizedUsageKind::Action,
            range: Some(subaction.syntax().text_range()),
            name_range,
            short_name_range,
            doc: None,
            relationships,
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    /// Create a NormalizedUsage from a ControlNode (fork/join/merge/decide)
    pub(super) fn from_control_node(node: &parser::ControlNode) -> Self {
        // Determine kind based on control node type
        let kind = match node.kind() {
            Some(parser::SyntaxKind::FORK_KW) => NormalizedUsageKind::Fork,
            Some(parser::SyntaxKind::JOIN_KW) => NormalizedUsageKind::Join,
            Some(parser::SyntaxKind::MERGE_KW) => NormalizedUsageKind::Merge,
            Some(parser::SyntaxKind::DECIDE_KW) => NormalizedUsageKind::Decide,
            _ => NormalizedUsageKind::Other,
        };

        // Extract name if present
        let name = node.name().and_then(|n| n.text());
        let short_name = node
            .name()
            .and_then(|n| n.short_name())
            .and_then(|sn| sn.text());
        let name_range = node.name().map(|n| n.syntax().text_range());
        let short_name_range = node
            .name()
            .and_then(|n| n.short_name())
            .map(|sn| sn.syntax().text_range());

        // Extract children from the body (if any)
        let children: Vec<NormalizedElement> = node
            .body()
            .map(|body| {
                body.members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            name,
            short_name,
            kind,
            range: Some(node.syntax().text_range()),
            name_range,
            short_name_range,
            doc: parser::extract_doc_comment(node.syntax()),
            relationships: Vec::new(),
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    /// Create a normalized usage from a for loop action usage.
    /// The for loop variable becomes a child usage.
    pub(super) fn from_for_loop(for_loop: &parser::ForLoopActionUsage) -> Self {
        // For loop variable becomes a child
        let mut children: Vec<NormalizedElement> = Vec::new();

        // Create a synthetic usage for the loop variable (e.g., `n : Integer`)
        if let Some(var_name) = for_loop.variable_name() {
            let name_text = var_name.text();

            // Extract typing if present
            let mut relationships = Vec::new();
            if let Some(typing) = for_loop.typing() {
                if let Some(target) = typing.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TypedBy,
                        target: RelTarget::Simple(target.to_string()),
                        range: Some(target.syntax().text_range()),
                    });
                }
            }

            children.push(NormalizedElement::Usage(NormalizedUsage {
                name: name_text.clone(),
                short_name: None,
                kind: NormalizedUsageKind::Attribute, // Loop variable is like an attribute
                relationships,
                range: Some(var_name.syntax().text_range()),
                name_range: Some(var_name.syntax().text_range()),
                short_name_range: None,
                doc: None,
                children: Vec::new(),
                is_abstract: false,
                is_variation: false,
                is_readonly: false,
                is_derived: false,
                is_parallel: false,
                is_individual: false,
                is_end: false,
                is_default: false,
                is_ordered: false,
                is_nonunique: false,
                is_portion: false,
                direction: None,
                multiplicity: None,
                value: None,
            }));
        }

        // Add members from the for loop body
        if let Some(body) = for_loop.body() {
            for member in body.members() {
                children.push(NormalizedElement::from_rowan(&member));
            }
        }

        Self {
            name: None, // For loops are anonymous
            short_name: None,
            kind: NormalizedUsageKind::Action, // For loop is an action
            range: Some(for_loop.syntax().text_range()),
            name_range: None,
            short_name_range: None,
            doc: parser::extract_doc_comment(for_loop.syntax()),
            relationships: Vec::new(),
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    /// Create a normalized usage from an if action usage.
    /// Extracts expression refs from condition and then/else targets.
    pub(super) fn from_if_action(if_action: &parser::IfActionUsage) -> Self {
        let mut relationships = Vec::new();
        let mut children = Vec::new();

        // Extract expression references from condition
        for expr in if_action.expressions() {
            extract_expression_chains(&expr, &mut relationships);
        }

        // Extract qualified name references (then/else action targets)
        for qn in if_action.qualified_names() {
            let name = qn.to_string();
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Expression,
                target: RelTarget::Simple(name),
                range: Some(qn.syntax().text_range()),
            });
        }

        // Add members from the if action body (if it has one)
        if let Some(body) = if_action.body() {
            for member in body.members() {
                children.push(NormalizedElement::from_rowan(&member));
            }
        }

        Self {
            name: None,
            short_name: None,
            kind: NormalizedUsageKind::Action,
            range: Some(if_action.syntax().text_range()),
            name_range: None,
            short_name_range: None,
            doc: parser::extract_doc_comment(if_action.syntax()),
            relationships,
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    /// Create a normalized usage from a while loop action usage.
    pub(super) fn from_while_loop(while_loop: &parser::WhileLoopActionUsage) -> Self {
        let mut relationships = Vec::new();
        let mut children = Vec::new();

        // Extract expression references from condition
        for expr in while_loop.expressions() {
            extract_expression_chains(&expr, &mut relationships);
        }

        // Add members from the while loop body
        if let Some(body) = while_loop.body() {
            for member in body.members() {
                children.push(NormalizedElement::from_rowan(&member));
            }
        }

        Self {
            name: None,
            short_name: None,
            kind: NormalizedUsageKind::Action,
            range: Some(while_loop.syntax().text_range()),
            name_range: None,
            short_name_range: None,
            doc: parser::extract_doc_comment(while_loop.syntax()),
            relationships,
            children,
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }
}

