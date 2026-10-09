pub struct WorkerTemplateOptions<'a> {
    pub supabase_url: Option<&'a str>,
    pub supabase_service_role_key: Option<&'a str>,
    pub webhook_secret: Option<&'a str>,
    pub local_webhook_url: Option<&'a str>,
}

pub fn generate_cloudflare_worker_script(opts: &WorkerTemplateOptions) -> String {
    let default_sb_url = opts.supabase_url.unwrap_or("https://<YOUR_PROJECT_ID>.supabase.co");
    let default_sb_key = opts.supabase_service_role_key.unwrap_or("<YOUR_SERVICE_ROLE_KEY>");
    let default_secret = opts.webhook_secret.unwrap_or("specter-inbox-shared-secret");
    let default_local_webhook = opts.local_webhook_url.unwrap_or("");

    format!(r#"/**
 * Specter Catch-All Email & OTP Interceptor Worker
 * Compatible with Cloudflare Email Routing & Workers
 * 
 * Deployment:
 * 1. Create a Cloudflare Worker (e.g. 'specter-inbox-router').
 * 2. Paste this script into the worker editor.
 * 3. Set Environment Secrets in Cloudflare Worker Settings:
 *    - SUPABASE_URL = "{default_sb_url}"
 *    - SUPABASE_SERVICE_ROLE_KEY = "{default_sb_key}"
 *    - WEBHOOK_SECRET = "{default_secret}"
 *    - LOCAL_WEBHOOK_URL = "{default_local_webhook}" (Optional, for direct tunnel)
 * 4. In Cloudflare Dashboard -> Email Routing -> Routing Rules:
 *    - Catch-all rule: Send to Worker -> specter-inbox-router
 */

export default {{
  async email(message, env, ctx) {{
    const from = message.from;
    const to = message.to;
    const subject = message.headers.get("subject") || "";
    const messageId = message.headers.get("message-id") || `msg-${{Date.now()}}-${{Math.random().toString(36).substring(2, 9)}}`;

    // Read full raw email content stream
    const rawStream = new Response(message.raw);
    const rawText = await rawStream.text();

    // Extract headers as JSON map
    const headersMap = {{}};
    for (const [key, val] of message.headers.entries()) {{
      headersMap[key.toLowerCase()] = val;
    }}

    // Basic plain text and HTML extractor from multipart MIME stream
    const {{ bodyText, bodyHtml }} = extractEmailBody(rawText);

    const payload = {{
      p_message_id: messageId,
      p_sender: from,
      p_recipient: to,
      p_subject: subject,
      p_body_text: bodyText || rawText.substring(0, 4000),
      p_body_html: bodyHtml || null,
      p_raw_headers: JSON.stringify(headersMap)
    }};

    const tasks = [];

    // Target 1: Supabase Cloud Ingestion (SSOT Database & Realtime Broadcast)
    const sbUrl = env.SUPABASE_URL || "{default_sb_url}";
    const sbKey = env.SUPABASE_SERVICE_ROLE_KEY || "{default_sb_key}";

    if (sbUrl && !sbUrl.includes("<YOUR_PROJECT_ID>") && sbKey && !sbKey.includes("<YOUR_SERVICE_ROLE_KEY>")) {{
      const sbPromise = fetch(`${{sbUrl}}/rest/v1/rpc/ingest_email`, {{
        method: "POST",
        headers: {{
          "Content-Type": "application/json",
          "apikey": sbKey,
          "Authorization": `Bearer ${{sbKey}}`,
          "Prefer": "return=representation"
        }},
        body: JSON.stringify(payload)
      }}).then(async (res) => {{
        if (!res.ok) {{
          const err = await res.text();
          console.error(`[Supabase Ingest Error] HTTP ${{res.status}}: ${{err}}`);
        }} else {{
          console.log(`[Supabase Ingest Success] Delivered to ${{to}}`);
        }}
      }}).catch((err) => {{
        console.error(`[Supabase Ingest Fetch Exception]`, err);
      }});

      tasks.push(sbPromise);
    }}

    // Target 2: Direct Local Workstation Webhook (if configured)
    const localWebhook = env.LOCAL_WEBHOOK_URL || "{default_local_webhook}";
    if (localWebhook) {{
      const secret = env.WEBHOOK_SECRET || "{default_secret}";
      const localPromise = fetch(localWebhook, {{
        method: "POST",
        headers: {{
          "Content-Type": "application/json",
          "X-Specter-Secret": secret
        }},
        body: JSON.stringify({{
          id: messageId,
          from: from,
          to: to,
          subject: subject,
          body_text: bodyText,
          body_html: bodyHtml,
          headers: headersMap
        }})
      }}).then((res) => {{
        console.log(`[Local Webhook Deliver] HTTP ${{res.status}}`);
      }}).catch((err) => {{
        console.warn(`[Local Webhook Unreachable]`, err.message);
      }});

      tasks.push(localPromise);
    }}

    if (tasks.length > 0) {{
      ctx.waitUntil(Promise.all(tasks));
    }}
  }}
}};

/**
 * Lightweight MIME body parser without external npm dependencies
 */
function extractEmailBody(raw) {{
  let bodyText = "";
  let bodyHtml = "";

  // Check if single-part plain text
  const parts = raw.split(/\r?\n\r?\n/);
  if (parts.length > 1) {{
    const bodyCandidate = parts.slice(1).join("\n\n");
    if (raw.includes("Content-Type: text/html")) {{
      bodyHtml = bodyCandidate;
    }} else {{
      bodyText = bodyCandidate;
    }}
  }} else {{
    bodyText = raw;
  }}

  return {{ bodyText: bodyText.trim(), bodyHtml: bodyHtml.trim() }};
}}
"#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_worker_script() {
        let opts = WorkerTemplateOptions {
            supabase_url: Some("https://example.supabase.co"),
            supabase_service_role_key: Some("test-key-12345"),
            webhook_secret: Some("secret-abc"),
            local_webhook_url: None,
        };

        let script = generate_cloudflare_worker_script(&opts);
        assert!(script.contains("https://example.supabase.co"));
        assert!(script.contains("test-key-12345"));
        assert!(script.contains("secret-abc"));
        assert!(script.contains("ingest_email"));
    }
}
