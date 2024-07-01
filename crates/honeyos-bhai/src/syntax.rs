use rhai::{Dynamic, Engine};

/// Register the custom syntax ontop of the rhai language.
/// The functionality of the custom syntax is determined by the kernel.
pub fn register_syntax(engine: &mut Engine) -> anyhow::Result<()> {
    register_echo(engine)?;
    register_cwd(engine)?;
    register_cd(engine)?;
    register_ls(engine)?;
    register_mkdir(engine)?;
    register_rm(engine)?;
    register_touch(engine)?;
    register_cat(engine)?;
    Ok(())
}

/// Register the echo keyword
fn register_echo(engine: &mut Engine) -> anyhow::Result<()> {
    engine
        .register_custom_syntax(["echo", "$expr$"], false, |context, inputs| {
            let string_expr = inputs
                .get(0)
                .ok_or("echo statement requires an input".to_string())?;
            let evaluated = string_expr.eval_with_context(context)?;
            let as_string = evaluated.to_string();
            let eval = format!("__keyword_echo(\"{}\")", as_string);
            context.engine().run(&eval)?;
            Ok(Dynamic::UNIT)
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Register the cwd keyword
fn register_cwd(engine: &mut Engine) -> anyhow::Result<()> {
    engine
        .register_custom_syntax(["cwd"], false, |context, _| {
            let eval = format!("__keyword_cwd()");
            let result = context.engine().eval::<String>(&eval)?;
            Ok(Dynamic::from(result))
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Register the cd keyword
fn register_cd(engine: &mut Engine) -> anyhow::Result<()> {
    engine
        .register_custom_syntax(["cd", "$expr$"], false, |context, inputs| {
            let string_expr = inputs
                .get(0)
                .ok_or("cd statement requires an input".to_string())?;
            let evaluated = string_expr.eval_with_context(context)?;
            let as_string = evaluated.to_string();
            let eval = format!("__keyword_cd(\"{}\")", as_string);
            context.engine().run(&eval)?;
            Ok(Dynamic::UNIT)
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Register the ls keyword
fn register_ls(engine: &mut Engine) -> anyhow::Result<()> {
    engine
        .register_custom_syntax(["ls"], false, |context, _| {
            let eval = format!("__keyword_ls()");
            let result = context.engine().eval::<String>(&eval)?;
            Ok(Dynamic::from(result))
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Register the mkdir keyword
fn register_mkdir(engine: &mut Engine) -> anyhow::Result<()> {
    engine
        .register_custom_syntax(["mkdir", "$expr$"], false, |context, inputs| {
            let string_expr = inputs
                .get(0)
                .ok_or("mkdir statement requires an input".to_string())?;
            let evaluated = string_expr.eval_with_context(context)?;
            let as_string = evaluated.to_string();
            let eval = format!("__keyword_mkdir(\"{}\")", as_string);
            context.engine().run(&eval)?;
            Ok(Dynamic::UNIT)
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Register the rm keyword
fn register_rm(engine: &mut Engine) -> anyhow::Result<()> {
    engine
        .register_custom_syntax(["rm", "$expr$"], false, |context, inputs| {
            let string_expr = inputs
                .get(0)
                .ok_or("rm statement requires an input".to_string())?;
            let evaluated = string_expr.eval_with_context(context)?;
            let as_string = evaluated.to_string();
            let eval = format!("__keyword_rm(\"{}\")", as_string);
            context.engine().run(&eval)?;
            Ok(Dynamic::UNIT)
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Register the touch keyword
fn register_touch(engine: &mut Engine) -> anyhow::Result<()> {
    engine
        .register_custom_syntax(["touch", "$expr$"], false, |context, inputs| {
            let string_expr = inputs
                .get(0)
                .ok_or("touch statement requires an input".to_string())?;
            let evaluated = string_expr.eval_with_context(context)?;
            let as_string = evaluated.to_string();
            let eval = format!("__keyword_touch(\"{}\")", as_string);
            context.engine().run(&eval)?;
            Ok(Dynamic::UNIT)
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Register the cat keyword
fn register_cat(engine: &mut Engine) -> anyhow::Result<()> {
    engine
        .register_custom_syntax(["cat", "$expr$"], false, |context, inputs| {
            let string_expr = inputs
                .get(0)
                .ok_or("cat statement requires an input".to_string())?;
            let evaluated = string_expr.eval_with_context(context)?;
            let as_string = evaluated.to_string();
            let eval = format!("__keyword_cat(\"{}\")", as_string);
            context.engine().run(&eval)?;
            Ok(Dynamic::UNIT)
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
