        const MAX_DEPTH: usize = 100;
        if depth > MAX_DEPTH {
            eprintln!("Warning: import recursion depth limit ({}) exceeded at depth {} for scope '{}'", MAX_DEPTH, depth, scope);
            return;
        }
        