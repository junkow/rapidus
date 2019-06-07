use crate::vm::{
    frame,
    jsvalue::value::*,
    vm::{Factory, VMResult, VM},
};

pub fn symbol(factory: &mut Factory) -> Value {
    let obj = factory.generate_builtin_constructor(
        "Symbol",
        symbol_constructor,
        factory.object_prototypes.symbol,
    );

    // Symbol.for
    obj.set_property_by_string_key("for", { factory.builtin_function("for", symbol_for) });
    // Symbol.keyFor
    obj.set_property_by_string_key("keyFor", {
        factory.builtin_function("keyFor", symbol_key_for)
    });
    obj
}

pub fn symbol_constructor(
    vm: &mut VM,
    args: &[Value],
    _this: Value,
    _cur_frame: &mut frame::Frame,
) -> VMResult {
    let symbol = vm.factory.symbol(args.get(0).map(|arg| arg.to_string()));
    vm.stack.push(symbol.into());
    Ok(())
}

pub fn symbol_for(
    vm: &mut VM,
    args: &[Value],
    _this: Value,
    _cur_frame: &mut frame::Frame,
) -> VMResult {
    let sym = vm.global_symbol_registry.for_(
        &mut vm.factory,
        args.get(0).unwrap_or(&Value::undefined()).to_string(),
    );
    vm.stack.push(sym.into());
    Ok(())
}

pub fn symbol_key_for(
    vm: &mut VM,
    args: &[Value],
    _this: Value,
    cur_frame: &mut frame::Frame,
) -> VMResult {
    let sym = args.get(0).map(|x| *x).unwrap_or(Value::undefined());

    if !sym.is_symbol() {
        return Err(cur_frame.error_type(format!("{} is not symbol", sym.debug_string(true))));
    }

    let key = vm.global_symbol_registry.key_for(&mut vm.factory, sym);
    vm.stack.push(key.into());
    Ok(())
}
