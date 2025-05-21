use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext, RenderError,
    RenderErrorReason,
};

/// Format a number with thousands separators
pub fn format_number(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0).ok_or_else(|| {
        RenderError::from(RenderErrorReason::ParamNotFoundForName(
            "format_number",
            "first parameter".to_string(),
        ))
    })?;

    let number = param.value().as_f64().ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "Parameter must be a number".to_string(),
        ))
    })?;

    out.write(&format!("{}", number as i64))?;
    Ok(())
}

/// Calculate and format time spent per visit
pub fn format_time_spent(total_time: f64, visits: f64) -> String {
    if visits <= 0.0 {
        return "0m 0s".to_string();
    }

    let seconds = total_time / visits;
    let minutes = (seconds / 60.0) as i64;
    let remaining_seconds = (seconds % 60.0) as i64;

    if minutes > 0 {
        format!("{minutes}m {remaining_seconds}s")
    } else {
        format!("{remaining_seconds}s")
    }
}

/// Calculate percentage and ensure it's between 0-100
pub fn percentage(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let value = h.param(0).and_then(|v| v.value().as_f64()).unwrap_or(0.0);
    let total = h.param(1).and_then(|v| v.value().as_f64()).unwrap_or(1.0);

    let percentage = if total > 0.0 {
        (value / total * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    out.write(&format!("{percentage:.1}"))?;
    Ok(())
}

/// Format a float with specified decimal places
pub fn format_float(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0).ok_or_else(|| {
        RenderError::from(RenderErrorReason::ParamNotFoundForName(
            "format_float",
            "first parameter".to_string(),
        ))
    })?;

    let number = param.value().as_f64().ok_or_else(|| {
        RenderError::from(RenderErrorReason::Other(
            "Parameter must be a number".to_string(),
        ))
    })?;

    let decimals = h.param(1).and_then(|v| v.value().as_u64()).unwrap_or(2) as usize;

    out.write(&format!("{number:.decimals$}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_spent() {
        assert_eq!(format_time_spent(0.0, 0.0), "0m 0s");
        assert_eq!(format_time_spent(30.0, 1.0), "30s");
        assert_eq!(format_time_spent(90.0, 1.0), "1m 30s");
        assert_eq!(format_time_spent(3600.0, 1.0), "60m 0s");
    }

    #[test]
    fn test_handlebars_helpers() {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("formatNumber", Box::new(format_number));
        handlebars.register_helper("percentage", Box::new(percentage));
        handlebars.register_helper("formatFloat", Box::new(format_float));

        // Test formatNumber
        let template = "{{formatNumber number}}";
        let mut data = serde_json::json!({"number": 1234.56});
        assert_eq!(handlebars.render_template(template, &data).unwrap(), "1234");

        // Test percentage
        let template = "{{percentage value total}}";
        data = serde_json::json!({
            "value": 25.0,
            "total": 100.0
        });
        assert_eq!(handlebars.render_template(template, &data).unwrap(), "25.0");

        // Test formatFloat
        let template = "{{formatFloat number 3}}";
        data = serde_json::json!({"number": 1234.5678});
        assert_eq!(
            handlebars.render_template(template, &data).unwrap(),
            "1234.568"
        );
    }
}
