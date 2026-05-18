use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde_json::Value;

use crate::model::*;
use crate::report;
use crate::youtrack::YouTrack;

#[derive(Clone)]
pub struct Server {
    yt: Arc<YouTrack>,
}

/// Drop `$type` discriminator keys YouTrack stamps on every object — pure
/// noise for an LLM consumer and ~10-15% of response tokens.
fn strip_noise(v: &mut Value) {
    match v {
        Value::Object(map) => {
            map.remove("$type");
            for child in map.values_mut() {
                strip_noise(child);
            }
        }
        Value::Array(arr) => arr.iter_mut().for_each(strip_noise),
        _ => {}
    }
}

fn ok(mut v: Value) -> Result<String, ErrorData> {
    strip_noise(&mut v);
    Ok(serde_json::to_string(&v).unwrap_or_else(|_| "null".into()))
}

fn req<'a>(v: &'a Option<String>, msg: &str) -> Result<&'a str, ErrorData> {
    v.as_deref()
        .ok_or_else(|| ErrorData::invalid_params(msg.to_string(), None))
}

#[tool_router]
impl Server {
    pub fn new(yt: Arc<YouTrack>) -> Self {
        Self { yt }
    }

    #[tool(description = "Create, update or delete an issue: summary, description, parentId (native subtask), assignee, tags (must exist), state, board/sprint. board+sprint resolve by name or id, any language/casing; a board exposes only its own sprints, so a wrong sprint errors with the valid sprint list for that board. op=delete is irreversible.")]
    async fn issue_write(&self, Parameters(a): Parameters<IssueWrite>) -> Result<String, ErrorData> {
        let v = match a.op {
            IssueOp::Create => self.yt.issue_create(&a).await?,
            IssueOp::Update => self.yt.issue_update(&a).await?,
            IssueOp::Delete => {
                self.yt
                    .issue_delete(req(&a.id, "issue_write op=delete requires 'id'")?)
                    .await?
            }
        };
        ok(v)
    }

    #[tool(description = "Get a single issue by id with full fields.")]
    async fn issue_get(&self, Parameters(a): Parameters<IdArg>) -> Result<String, ErrorData> {
        ok(self.yt.issue_get(&a.id).await?)
    }

    #[tool(description = "Search issues by YouTrack query. fields short|full.")]
    async fn issue_search(&self, Parameters(a): Parameters<IssueSearch>) -> Result<String, ErrorData> {
        let full = matches!(a.fields, Some(SearchFields::Full));
        ok(self
            .yt
            .issue_search(&a.query, full, a.top.unwrap_or(50), a.skip.unwrap_or(0))
            .await?)
    }

    #[tool(description = "List links of an issue (direction, type, related issues).")]
    async fn issue_links(&self, Parameters(a): Parameters<IdArg>) -> Result<String, ErrorData> {
        ok(self.yt.issue_links(&a.id).await?)
    }

    #[tool(description = "Add or remove an issue link. role outward|inward for directed types. For parent/child use issue_write.parentId.")]
    async fn link_write(&self, Parameters(a): Parameters<LinkWrite>) -> Result<String, ErrorData> {
        let inward = matches!(a.role, Some(LinkRole::Inward));
        let v = match a.op {
            LinkOp::Add => {
                self.yt
                    .link_add(&a.source_id, &a.target_id, &a.link_type, inward)
                    .await?
            }
            LinkOp::Remove => {
                self.yt
                    .link_remove(&a.source_id, &a.target_id, &a.link_type, inward)
                    .await?
            }
        };
        ok(v)
    }

    #[tool(description = "Create or update a comment on an issue or article (entity). op=update needs commentId.")]
    async fn comment_write(&self, Parameters(a): Parameters<CommentWrite>) -> Result<String, ErrorData> {
        let comment_id = match a.op {
            WriteOp::Create => None,
            WriteOp::Update => Some(req(&a.comment_id, "comment_write op=update requires 'commentId'")?),
        };
        ok(self
            .yt
            .comment_write(
                a.entity,
                &a.parent_id,
                comment_id,
                &a.text,
                a.markdown,
                a.mute.unwrap_or(false),
            )
            .await?)
    }

    #[tool(description = "List comments of an issue or article (entity).")]
    async fn comments_list(&self, Parameters(a): Parameters<CommentsList>) -> Result<String, ErrorData> {
        ok(self.yt.comments_list(a.entity, &a.parent_id).await?)
    }

    #[tool(description = "Create or update a knowledge-base article.")]
    async fn article_write(&self, Parameters(a): Parameters<ArticleWrite>) -> Result<String, ErrorData> {
        let v = match a.op {
            WriteOp::Create => self.yt.article_create(&a).await?,
            WriteOp::Update => self.yt.article_update(&a).await?,
        };
        ok(v)
    }

    #[tool(description = "Get an article by id (op get) or list articles (op list, optional query).")]
    async fn article_get(&self, Parameters(a): Parameters<ArticleGet>) -> Result<String, ErrorData> {
        let v = match a.op {
            GetOp::Get => {
                self.yt
                    .article_get(req(&a.id, "article_get op=get requires 'id'")?)
                    .await?
            }
            GetOp::List => self.yt.article_list(a.query.as_deref()).await?,
        };
        ok(v)
    }

    #[tool(description = "Create/update/delete a time-tracking work item. type = work item type name/id. idempotent skips duplicates.")]
    async fn workitem_write(&self, Parameters(a): Parameters<WorkitemWrite>) -> Result<String, ErrorData> {
        let v = match a.op {
            WorkOp::Create => self.yt.workitem_create(&a).await?,
            WorkOp::Update => self.yt.workitem_update(&a).await?,
            WorkOp::Delete => {
                let wid = req(&a.work_item_id, "workitem_write op=delete requires 'workItemId'")?;
                self.yt.workitem_delete(&a.issue_id, wid).await?
            }
        };
        ok(v)
    }

    #[tool(description = "List/aggregate work items by author and date range.")]
    async fn workitems_list(&self, Parameters(a): Parameters<WorkitemsList>) -> Result<String, ErrorData> {
        ok(self
            .yt
            .workitems_list(
                a.author.as_deref(),
                a.start_date.as_deref(),
                a.end_date.as_deref(),
                a.issue_id.as_deref(),
                a.top.unwrap_or(200),
                a.skip.unwrap_or(0),
            )
            .await?)
    }

    #[tool(description = "Per-day expected-vs-actual worktime report (480m/day, skips weekends/holidays).")]
    async fn workitems_report(&self, Parameters(a): Parameters<WorkitemsReport>) -> Result<String, ErrorData> {
        ok(report::workitems_report(&self.yt, a.author.as_deref(), &a.start_date, &a.end_date).await?)
    }

    #[tool(description = "Users: op list (optional query) | me | get (by id).")]
    async fn users(&self, Parameters(a): Parameters<UsersArg>) -> Result<String, ErrorData> {
        let v = match a.op {
            UsersOp::List => self.yt.users_list(a.query.as_deref()).await?,
            UsersOp::Me => self.yt.user_current().await?,
            UsersOp::Get => {
                self.yt
                    .user_get(req(&a.id, "users op=get requires 'id'")?)
                    .await?
            }
        };
        ok(v)
    }

    #[tool(description = "Discovery: kind projects | link_types | work_item_types (optional project).")]
    async fn meta(&self, Parameters(a): Parameters<MetaArg>) -> Result<String, ErrorData> {
        let v = match a.kind {
            MetaKind::Projects => self.yt.meta_projects().await?,
            MetaKind::LinkTypes => self.yt.meta_link_types().await?,
            MetaKind::WorkItemTypes => self.yt.meta_work_types(a.project.as_deref()).await?,
        };
        ok(v)
    }

    #[tool(description = "Activity feed. scope=issue (needs issueId, optional author) | user (needs author, defaults last 30d). categories default CustomFieldCategory,CommentsCategory. Dates ISO or unix ms.")]
    async fn activity(&self, Parameters(a): Parameters<ActivityArg>) -> Result<String, ErrorData> {
        let cats = a.categories.as_deref();
        let top = a.top.unwrap_or(100);
        let skip = a.skip.unwrap_or(0);
        let v = match a.scope {
            ActivityScope::Issue => {
                let issue = req(&a.issue_id, "activity scope=issue requires 'issueId'")?;
                self.yt
                    .issue_activities(
                        issue,
                        a.author.as_deref(),
                        a.start_date.as_deref(),
                        a.end_date.as_deref(),
                        cats,
                        top,
                        skip,
                    )
                    .await?
            }
            ActivityScope::User => {
                let author = req(&a.author, "activity scope=user requires 'author'")?;
                self.yt
                    .users_activity(
                        author,
                        a.start_date.as_deref(),
                        a.end_date.as_deref(),
                        cats,
                        a.reverse,
                        top,
                        skip,
                    )
                    .await?
            }
        };
        ok(v)
    }

    #[tool(description = "Issue attachments: op list|get|upload|download|delete. Upload via contentBase64; download returns base64 unless path given.")]
    async fn attachment(&self, Parameters(a): Parameters<AttachmentArg>) -> Result<String, ErrorData> {
        let v = match a.op {
            AttachOp::List => self.yt.attachments_list(&a.issue_id).await?,
            AttachOp::Get => {
                let aid = req(&a.attachment_id, "attachment op=get requires 'attachmentId'")?;
                self.yt.attachment_get(&a.issue_id, aid).await?
            }
            AttachOp::Upload => {
                let name = req(&a.name, "attachment op=upload requires 'name'")?;
                let bytes = if let Some(b64) = &a.content_base64 {
                    YouTrack::b64_decode(b64).map_err(ErrorData::from)?
                } else if let Some(p) = &a.path {
                    std::fs::read(p)
                        .map_err(|e| ErrorData::invalid_params(format!("read {p}: {e}"), None))?
                } else {
                    return Err(ErrorData::invalid_params("contentBase64 or path required", None));
                };
                self.yt.attachment_upload(&a.issue_id, name, bytes).await?
            }
            AttachOp::Download => {
                let aid = req(&a.attachment_id, "attachment op=download requires 'attachmentId'")?;
                let (name, bytes) = self.yt.attachment_download(&a.issue_id, aid).await?;
                let target = a.path.clone().or_else(|| {
                    self.yt
                        .cfg
                        .download_dir
                        .as_ref()
                        .map(|d| format!("{}/{}", d.trim_end_matches('/'), name))
                });
                match target {
                    Some(path) => {
                        std::fs::write(&path, &bytes)
                            .map_err(|e| ErrorData::internal_error(format!("write {path}: {e}"), None))?;
                        serde_json::json!({"saved": path, "bytes": bytes.len()})
                    }
                    None => serde_json::json!({
                        "name": name,
                        "bytes": bytes.len(),
                        "contentBase64": YouTrack::b64_encode(&bytes)
                    }),
                }
            }
            AttachOp::Delete => {
                let aid = req(&a.attachment_id, "attachment op=delete requires 'attachmentId'")?;
                self.yt.attachment_delete(&a.issue_id, aid).await?
            }
        };
        ok(v)
    }
}

#[tool_handler(
    name = "youtrack-mcp",
    version = "0.1.0",
    instructions = "Lean YouTrack MCP. Conventions: issue ids are readable like ABC-123 (bare numbers expand via YOUTRACK_DEFAULT_PROJECT). For parent/child use issue_write.parentId (native subtask) — NOT link_write. link_write needs sourceId+targetId+linkType (name e.g. 'Relates','Depend') and role outward|inward for directed types. Dates are ISO YYYY-MM-DD. tags must already exist (this server never creates tags; unknown tag = error). issue_write op=delete permanently deletes the issue. workitem_write needs date+minutes (+type name like 'Разработка'); idempotent=true skips a same issue+date+description entry. workitems_list/report default to the current user. Errors are returned as JSON-RPC errors whose message states the missing/invalid field and the fix; 'YouTrack <status>: <msg>' means the API rejected the call."
)]
impl ServerHandler for Server {}
