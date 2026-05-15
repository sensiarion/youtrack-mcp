use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;
use reqwest::{Client, Method, StatusCode};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::config::Config;
use crate::error::{AppError, Result};

const F_ISSUE: &str = "id,idReadable,summary,description,usesMarkdown,created,updated,resolved,project(id,shortName,name),parent(issues(idReadable,summary)),assignee(id,login,name),reporter(id,login,name),tags(id,name),customFields(id,name,value(id,login,name,presentation),$type)";
const F_ISSUE_SHORT: &str = "id,idReadable,summary,project(shortName),customFields(name,value(name,login,presentation),$type)";
const F_ISSUE_FULL: &str = "id,idReadable,summary,description,usesMarkdown,project(id,shortName,name),parent(issues(idReadable,summary)),assignee(login,name),tags(name),customFields(name,value(name,login,presentation),$type)";
const F_LINKS: &str = "id,direction,linkType(id,name,directed,sourceToTarget,targetToSource),issues(idReadable,summary,project(shortName),assignee(login,name))";
const F_LINK_TYPES: &str = "id,name,directed,sourceToTarget,targetToSource,aggregation";
const F_COMMENT: &str = "id,text,usesMarkdown,author(id,login,name),created,updated";
const F_ARTICLE: &str = "id,idReadable,summary,content,usesMarkdown,parentArticle(id,idReadable),project(id,shortName,name)";
const F_ARTICLE_LIST: &str = "id,idReadable,summary,parentArticle(id,idReadable),project(shortName)";
const F_WORKITEM: &str = "id,date,updated,duration(minutes,presentation),text,description,usesMarkdown,type(id,name),issue(id,idReadable),author(id,login,name)";
const F_USERS: &str = "id,login,name,fullName,email";
const F_PROJECTS: &str = "id,shortName,name";
const F_ATTACH: &str = "id,name,author(login),created,size,mimeType,url,extension";
const F_ACTIVITY: &str = "id,timestamp,author(login,name),category(id),target(text,issue(idReadable,summary)),added(name,login),removed(name,login)";
const ACTIVITY_DEFAULT_CATEGORIES: &str = "CustomFieldCategory,CommentsCategory";

pub struct YouTrack {
    pub cfg: Config,
    http: Client,
    link_types: RwLock<Vec<Value>>,
    projects: RwLock<HashMap<String, String>>,
    work_types: RwLock<HashMap<String, String>>,
    tags: RwLock<HashMap<String, String>>,
    current_login: RwLock<Option<String>>,
}

fn is_id(s: &str) -> bool {
    let mut parts = s.split('-');
    matches!((parts.next(), parts.next(), parts.next()),
        (Some(a), Some(b), None) if !a.is_empty() && !b.is_empty()
            && a.bytes().all(|c| c.is_ascii_digit())
            && b.bytes().all(|c| c.is_ascii_digit()))
}


fn require<'a>(opt: Option<&'a String>, msg: &str) -> Result<&'a str> {
    opt.map(|s| s.as_str())
        .ok_or_else(|| AppError::Bad(msg.to_string()))
}

impl YouTrack {
    pub fn new(cfg: Config) -> Result<Arc<Self>> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", cfg.token)
                .parse()
                .map_err(|_| AppError::Config("invalid token".into()))?,
        );
        headers.insert(reqwest::header::ACCEPT, "application/json".parse().unwrap());
        let http = Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(60))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(Arc::new(Self {
            cfg,
            http,
            link_types: RwLock::new(vec![]),
            projects: RwLock::new(HashMap::new()),
            work_types: RwLock::new(HashMap::new()),
            tags: RwLock::new(HashMap::new()),
            current_login: RwLock::new(None),
        }))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.cfg.base_url, path)
    }

    async fn check(&self, resp: reqwest::Response) -> Result<Value> {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.is_success() {
            if body.trim().is_empty() {
                return Ok(Value::Null);
            }
            serde_json::from_str(&body)
                .map_err(|e| AppError::Network(format!("bad JSON: {e}")))
        } else {
            let msg = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| {
                    for k in ["error_description", "message", "error", "code"] {
                        if let Some(s) = v.get(k).and_then(|x| x.as_str()) {
                            return Some(s.to_string());
                        }
                    }
                    None
                })
                .unwrap_or_else(|| body.chars().take(200).collect());
            Err(AppError::Api { status: status.as_u16(), message: msg })
        }
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        let has_page = query.iter().any(|(k, _)| *k == "$top" || *k == "$skip");
        // GETs are idempotent: retry once on transient connect/timeout failures.
        let resp = match self.http.get(self.url(path)).query(query).send().await {
            Ok(r) => r,
            Err(e) if e.is_timeout() || e.is_connect() => {
                self.http.get(self.url(path)).query(query).send().await?
            }
            Err(e) => return Err(e.into()),
        };
        if has_page && resp.status() == StatusCode::BAD_REQUEST {
            let plain: Vec<(String, String)> = query
                .iter()
                .map(|(k, v)| (k.trim_start_matches('$').to_string(), v.clone()))
                .collect();
            let resp2 = self.http.get(self.url(path)).query(&plain).send().await?;
            return self.check(resp2).await;
        }
        self.check(resp).await
    }

    async fn send_json(&self, method: Method, path: &str, query: &[(&str, String)], body: &Value) -> Result<Value> {
        let resp = self
            .http
            .request(method, self.url(path))
            .query(query)
            .json(body)
            .send()
            .await?;
        self.check(resp).await
    }

    async fn post(&self, path: &str, query: &[(&str, String)], body: &Value) -> Result<Value> {
        self.send_json(Method::POST, path, query, body).await
    }

    async fn delete(&self, path: &str) -> Result<Value> {
        let resp = self.http.delete(self.url(path)).send().await?;
        self.check(resp).await
    }

    async fn command(&self, query: &str, id_readable: &str) -> Result<()> {
        let body = json!({"query": query, "issues": [{"idReadable": id_readable}], "silent": true});
        self.post("/api/commands", &[], &body).await?;
        Ok(())
    }

    fn fq(&self, fields: &str) -> [(&'static str, String); 1] {
        [("fields", fields.to_string())]
    }

    // ---- resolvers / caches ----

    /// name→id resolver shared by projects/tags/work-item-types: bare ids pass
    /// through, otherwise fetch the list once and cache `key_field`→`id`.
    async fn cached_id(
        &self,
        cache: &RwLock<HashMap<String, String>>,
        endpoint: &str,
        fields: &str,
        key_field: &str,
        name: &str,
        noun: &str,
    ) -> Result<String> {
        if is_id(name) {
            return Ok(name.to_string());
        }
        if let Some(id) = cache.read().await.get(name) {
            return Ok(id.clone());
        }
        let list = self
            .get(endpoint, &[("fields", fields.to_string()), ("$top", "1000".to_string())])
            .await?;
        let mut map = cache.write().await;
        if let Some(arr) = list.as_array() {
            for e in arr {
                if let (Some(k), Some(id)) = (
                    e.get(key_field).and_then(|x| x.as_str()),
                    e.get("id").and_then(|x| x.as_str()),
                ) {
                    map.insert(k.to_string(), id.to_string());
                }
            }
        }
        map.get(name)
            .cloned()
            .ok_or_else(|| AppError::Bad(format!("unknown {noun} '{name}'")))
    }

    pub async fn project_id(&self, project: &str) -> Result<String> {
        self.cached_id(&self.projects, "/api/admin/projects", F_PROJECTS, "shortName", project, "project")
            .await
    }

    async fn user_value(&self, login: &str) -> Result<Value> {
        let login = self.cfg.resolve_alias(login).to_string();
        let q = [("fields", "id,login,name".to_string()), ("query", login.clone())];
        let res = self.get("/api/users", &q).await?;
        if let Some(arr) = res.as_array() {
            if let Some(u) = arr.iter().find(|u| u.get("login").and_then(|x| x.as_str()) == Some(&login)) {
                return Ok(json!({"id": u.get("id"), "login": login}));
            }
            if let Some(u) = arr.first() {
                return Ok(json!({"id": u.get("id"), "login": u.get("login")}));
            }
        }
        Err(AppError::Bad(format!("unknown user '{login}'")))
    }

    pub async fn work_type_id(&self, name_or_id: &str) -> Result<String> {
        self.cached_id(
            &self.work_types,
            "/api/admin/timeTrackingSettings/workItemTypes",
            "id,name",
            "name",
            name_or_id,
            "work item type",
        )
        .await
    }

    async fn link_type(&self, name: &str) -> Result<Value> {
        {
            let c = self.link_types.read().await;
            if let Some(t) = c.iter().find(|t| {
                t.get("name").and_then(|x| x.as_str()) == Some(name)
                    || t.get("id").and_then(|x| x.as_str()) == Some(name)
            }) {
                return Ok(t.clone());
            }
        }
        let list = self.get("/api/issueLinkTypes", &self.fq(F_LINK_TYPES)).await?;
        let arr = list.as_array().cloned().unwrap_or_default();
        *self.link_types.write().await = arr.clone();
        arr.into_iter()
            .find(|t| {
                t.get("name").and_then(|x| x.as_str()) == Some(name)
                    || t.get("id").and_then(|x| x.as_str()) == Some(name)
            })
            .ok_or_else(|| AppError::Bad(format!("unknown link type '{name}'")))
    }

    // ---- issues ----

    pub async fn issue_get(&self, id: &str) -> Result<Value> {
        let id = self.cfg.expand_issue_id(id);
        self.get(&format!("/api/issues/{id}"), &self.fq(F_ISSUE)).await
    }

    pub async fn issue_search(&self, query: &str, full: bool, top: i64, skip: i64) -> Result<Value> {
        let fields = if full { F_ISSUE_FULL } else { F_ISSUE_SHORT };
        let q = [
            ("fields", fields.to_string()),
            ("query", query.to_string()),
            ("$top", top.to_string()),
            ("$skip", skip.to_string()),
        ];
        self.get("/api/issues", &q).await
    }

    pub async fn issue_links(&self, id: &str) -> Result<Value> {
        let id = self.cfg.expand_issue_id(id);
        self.get(&format!("/api/issues/{id}/links"), &self.fq(F_LINKS)).await
    }

    /// Resolve an existing tag name to its id. This server never creates tags;
    /// an unknown name is an error.
    async fn tag_id(&self, name: &str) -> Result<String> {
        self.cached_id(&self.tags, "/api/tags", "id,name", "name", name, "tag (must already exist)")
            .await
    }

    async fn set_board(&self, id_readable: &str, board: &str, sprint: Option<&str>) -> Result<()> {
        let q = match sprint {
            Some(s) => format!("Board {board} {s}"),
            None => format!("Board {board}"),
        };
        self.command(&q, id_readable).await
    }

    pub async fn issue_delete(&self, id: &str) -> Result<Value> {
        let id = self.cfg.expand_issue_id(id);
        self.delete(&format!("/api/issues/{id}")).await?;
        Ok(json!({"deleted": true, "id": id}))
    }

    /// Parent/child on this on-prem is only honored via the command API
    /// (`subtask of <parent>` on the child); the issue `parent` body field is
    /// silently ignored. `parent=None` detaches from the current parent.
    async fn set_parent(&self, child_readable: &str, parent: Option<&str>) -> Result<()> {
        match parent {
            Some(p) => {
                let p = self.cfg.expand_issue_id(p);
                self.command(&format!("subtask of {p}"), child_readable).await
            }
            None => {
                let cur = self
                    .get(
                        &format!("/api/issues/{child_readable}"),
                        &self.fq("parent(issues(idReadable))"),
                    )
                    .await?;
                if let Some(p) = cur.pointer("/parent/issues/0/idReadable").and_then(|x| x.as_str()) {
                    self.command(&format!("remove subtask of {p}"), child_readable).await?;
                }
                Ok(())
            }
        }
    }

    pub async fn issue_create(&self, a: &crate::model::IssueWrite) -> Result<Value> {
        let project = require(
            a.project.as_ref(),
            "issue_write op=create requires 'project' (project shortName like ABC or its id)",
        )?;
        let mut body = json!({
            "summary": a.summary.clone().unwrap_or_default(),
            "project": {"id": self.project_id(project).await?},
        });
        if let Some(d) = &a.description {
            body["description"] = json!(d);
        }
        if let Some(m) = a.markdown {
            body["usesMarkdown"] = json!(m);
        }
        let created = self.post("/api/issues", &self.fq(F_ISSUE), &body).await?;
        let internal = created.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let readable =
            created.get("idReadable").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        if self.apply_side_effects(&internal, &readable, a).await? {
            self.get(&format!("/api/issues/{internal}"), &self.fq(F_ISSUE)).await
        } else {
            Ok(created)
        }
    }

    pub async fn issue_update(&self, a: &crate::model::IssueWrite) -> Result<Value> {
        let raw = require(
            a.id.as_ref(),
            "issue_write op=update requires 'id' (issue id like ABC-123)",
        )?;
        let id = self.cfg.expand_issue_id(raw);
        let mut body = json!({});
        if let Some(s) = &a.summary {
            body["summary"] = json!(s);
        }
        if let Some(d) = &a.description {
            body["description"] = json!(d);
        }
        if let Some(m) = a.markdown {
            body["usesMarkdown"] = json!(m);
        }
        let mut current = if body.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
            Some(self.post(&format!("/api/issues/{id}"), &self.fq(F_ISSUE), &body).await?)
        } else {
            None
        };
        if self.apply_side_effects(&id, &id, a).await? || current.is_none() {
            current = Some(self.get(&format!("/api/issues/{id}"), &self.fq(F_ISSUE)).await?);
        }
        Ok(current.unwrap())
    }

    /// Apply assignee/state/tags in a single issue POST and board via command.
    /// Returns true if anything was changed (caller then re-reads the issue).
    async fn apply_side_effects(&self, id: &str, readable: &str, a: &crate::model::IssueWrite) -> Result<bool> {
        let mut fields = Vec::new();
        if let Some(login) = &a.assignee {
            let mut user = self.user_value(login).await?;
            user["$type"] = json!("User");
            fields.push(json!({"name":"Assignee","$type":"SingleUserIssueCustomField","value":user}));
        }
        if let Some(state) = &a.state {
            fields.push(json!({"name":"State","$type":"StateIssueCustomField","value":{"name":state}}));
        }
        let mut body = json!({});
        if !fields.is_empty() {
            body["customFields"] = json!(fields);
        }
        if let Some(tags) = a.tags.as_ref().filter(|t| !t.is_empty()) {
            let mut refs = Vec::with_capacity(tags.len());
            for t in tags {
                refs.push(json!({"id": self.tag_id(t).await?}));
            }
            body["tags"] = json!(refs);
        }
        let mut changed = false;
        if body.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
            self.post(&format!("/api/issues/{id}"), &[], &body).await?;
            changed = true;
        }
        if let Some(p) = &a.parent_id {
            if p.is_empty() {
                self.set_parent(readable, None).await?;
            } else {
                self.set_parent(readable, Some(p)).await?;
            }
            changed = true;
        }
        if let Some(board) = &a.board {
            self.set_board(readable, board, a.sprint.as_deref()).await?;
            changed = true;
        }
        Ok(changed)
    }

    // ---- links ----

    fn link_keyword(lt: &Value, inward: bool) -> Result<String> {
        let directed = lt.get("directed").and_then(|x| x.as_bool()).unwrap_or(false);
        let s = lt.get("sourceToTarget").and_then(|x| x.as_str()).unwrap_or("");
        let t = lt.get("targetToSource").and_then(|x| x.as_str()).unwrap_or("");
        let k = if directed && inward { t } else { s };
        if k.is_empty() {
            Err(AppError::Bad("link type has no usable keyword".into()))
        } else {
            Ok(k.to_string())
        }
    }

    pub async fn link_add(&self, source: &str, target: &str, link_type: &str, inward: bool) -> Result<Value> {
        let source = self.cfg.expand_issue_id(source);
        let target = self.cfg.expand_issue_id(target);
        let lt = self.link_type(link_type).await?;
        let body = json!({"linkType": {"name": lt.get("name")}, "issues": [{"idReadable": target}]});
        match self
            .post(&format!("/api/issues/{source}/links"), &self.fq(F_LINKS), &body)
            .await
        {
            Ok(v) => Ok(v),
            Err(AppError::Api { status, .. }) if status == 404 || status == 405 => {
                let keyword = Self::link_keyword(&lt, inward)?;
                self.command(&format!("{keyword}: {target}"), &source).await?;
                Ok(json!({"linked": true, "source": source, "target": target, "via": "command"}))
            }
            Err(e) => Err(e),
        }
    }

    pub async fn link_remove(&self, source: &str, target: &str, link_type: &str, inward: bool) -> Result<Value> {
        let source = self.cfg.expand_issue_id(source);
        let target = self.cfg.expand_issue_id(target);
        let lt = self.link_type(link_type).await?;
        let keyword = Self::link_keyword(&lt, inward)?;
        self.command(&format!("remove {keyword}: {target}"), &source).await?;
        Ok(json!({"unlinked": true, "source": source, "target": target}))
    }

    // ---- comments ----

    fn comment_root(&self, entity: crate::model::Entity, parent: &str) -> String {
        match entity {
            crate::model::Entity::Issue => {
                format!("/api/issues/{}/comments", self.cfg.expand_issue_id(parent))
            }
            crate::model::Entity::Article => format!("/api/articles/{parent}/comments"),
        }
    }

    pub async fn comments_list(&self, entity: crate::model::Entity, parent: &str) -> Result<Value> {
        self.get(&self.comment_root(entity, parent), &self.fq(F_COMMENT)).await
    }

    /// Create (comment_id None) or update (Some) a comment on an issue/article.
    pub async fn comment_write(
        &self,
        entity: crate::model::Entity,
        parent: &str,
        comment_id: Option<&str>,
        text: &str,
        markdown: Option<bool>,
        mute: bool,
    ) -> Result<Value> {
        let mut body = json!({ "text": text });
        if let Some(m) = markdown {
            body["usesMarkdown"] = json!(m);
        }
        let root = self.comment_root(entity, parent);
        let path = match comment_id {
            Some(c) => format!("{root}/{c}"),
            None => root,
        };
        let mut q = vec![("fields", F_COMMENT.to_string())];
        if mute && comment_id.is_some() {
            q.push(("muteUpdateNotifications", "true".to_string()));
        }
        self.post(&path, &q, &body).await
    }

    // ---- articles ----

    pub async fn article_get(&self, id: &str) -> Result<Value> {
        self.get(&format!("/api/articles/{id}"), &self.fq(F_ARTICLE)).await
    }

    pub async fn article_list(&self, query: Option<&str>) -> Result<Value> {
        let mut q = vec![("fields", F_ARTICLE_LIST.to_string()), ("$top", "100".to_string())];
        if let Some(query) = query {
            q.push(("query", query.to_string()));
        }
        self.get("/api/articles", &q).await
    }

    pub async fn article_create(&self, a: &crate::model::ArticleWrite) -> Result<Value> {
        let project = a.project.as_deref().ok_or_else(|| {
            AppError::Bad(
                "article_write op=create requires 'project' (project shortName or id)".into(),
            )
        })?;
        let pid = self.project_id(project).await?;
        let mut body = json!({
            "summary": a.summary.clone().unwrap_or_default(),
            "content": a.content.clone().unwrap_or_default(),
            "project": {"id": pid},
        });
        if let Some(m) = a.markdown {
            body["usesMarkdown"] = json!(m);
        }
        if let Some(p) = &a.parent_article_id {
            body["parentArticle"] = json!({"id": p});
        }
        self.post("/api/articles", &self.fq(F_ARTICLE), &body).await
    }

    pub async fn article_update(&self, a: &crate::model::ArticleWrite) -> Result<Value> {
        let id = a.id.as_deref().ok_or_else(|| {
            AppError::Bad("article_write op=update requires 'id' (article id)".into())
        })?;
        let mut body = json!({});
        if let Some(s) = &a.summary {
            body["summary"] = json!(s);
        }
        if let Some(c) = &a.content {
            body["content"] = json!(c);
        }
        if let Some(m) = a.markdown {
            body["usesMarkdown"] = json!(m);
        }
        if let Some(p) = &a.parent_article_id {
            body["parentArticle"] = json!({"id": p});
        }
        self.post(&format!("/api/articles/{id}"), &self.fq(F_ARTICLE), &body).await
    }

    // ---- work items ----

    pub async fn workitems_list(
        &self,
        author: Option<&str>,
        start: Option<&str>,
        end: Option<&str>,
        issue: Option<&str>,
        top: i64,
        skip: i64,
    ) -> Result<Value> {
        let mut q = vec![
            ("fields", F_WORKITEM.to_string()),
            ("$top", top.to_string()),
            ("$skip", skip.to_string()),
        ];
        let author_login = match author {
            Some(a) => self.cfg.resolve_alias(a).to_string(),
            None => self.current_login().await?,
        };
        q.push(("author", author_login));
        if let Some(s) = start {
            q.push(("startDate", s.to_string()));
        }
        if let Some(e) = end {
            q.push(("endDate", e.to_string()));
        }
        if let Some(i) = issue {
            q.push(("issueId", self.cfg.expand_issue_id(i)));
        }
        self.get("/api/workItems", &q).await
    }

    fn date_to_epoch_ms(&self, iso: &str) -> Result<i64> {
        use chrono::TimeZone;
        let d = chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d")
            .map_err(|_| AppError::Bad(format!("bad date '{iso}', expected YYYY-MM-DD")))?;
        let dt = d.and_hms_opt(12, 0, 0).unwrap();
        let local = self
            .cfg
            .timezone
            .from_local_datetime(&dt)
            .single()
            .ok_or_else(|| AppError::Bad("ambiguous date".into()))?;
        Ok(local.timestamp_millis())
    }

    pub async fn workitem_create(&self, a: &crate::model::WorkitemWrite) -> Result<Value> {
        let issue = self.cfg.expand_issue_id(&a.issue_id);
        let date = a.date.as_deref().ok_or_else(|| {
            AppError::Bad("workitem_write op=create requires 'date' (ISO YYYY-MM-DD)".into())
        })?;
        let minutes = a.minutes.ok_or_else(|| {
            AppError::Bad("workitem_write op=create requires 'minutes' (integer duration)".into())
        })?;
        let desc = a.description.clone().or_else(|| a.text.clone()).unwrap_or_default();

        if a.idempotent.unwrap_or(false) {
            let existing = self
                .workitems_list(None, Some(date), Some(date), Some(&issue), 200, 0)
                .await?;
            if let Some(arr) = existing.as_array() {
                if arr.iter().any(|w| {
                    w.get("description").and_then(|x| x.as_str()) == Some(&desc)
                        || w.get("text").and_then(|x| x.as_str()) == Some(&desc)
                }) {
                    return Ok(json!({"skipped": true, "reason": "already logged"}));
                }
            }
        }

        let mut body = json!({
            "date": self.date_to_epoch_ms(date)?,
            "duration": {"minutes": minutes},
            "text": a.text.clone().unwrap_or_else(|| desc.clone()),
            "description": desc,
        });
        if let Some(m) = a.markdown {
            body["usesMarkdown"] = json!(m);
        }
        if let Some(t) = &a.work_type {
            body["type"] = json!({"id": self.work_type_id(t).await?});
        }
        self.post(
            &format!("/api/issues/{issue}/timeTracking/workItems"),
            &self.fq(F_WORKITEM),
            &body,
        )
        .await
    }

    pub async fn workitem_update(&self, a: &crate::model::WorkitemWrite) -> Result<Value> {
        let issue = self.cfg.expand_issue_id(&a.issue_id);
        let wid = a
            .work_item_id
            .as_deref()
            .ok_or_else(|| {
                AppError::Bad(
                    "workitem_write op=update requires 'workItemId' (get it from workitems_list)"
                        .into(),
                )
            })?;
        let mut body = json!({});
        if let Some(d) = &a.date {
            body["date"] = json!(self.date_to_epoch_ms(d)?);
        }
        if let Some(m) = a.minutes {
            body["duration"] = json!({"minutes": m});
        }
        if let Some(t) = &a.text {
            body["text"] = json!(t);
        }
        if let Some(d) = &a.description {
            body["description"] = json!(d);
        }
        if let Some(t) = &a.work_type {
            body["type"] = json!({"id": self.work_type_id(t).await?});
        }
        let path = format!("/api/issues/{issue}/timeTracking/workItems/{wid}");
        match self.post(&path, &self.fq(F_WORKITEM), &body).await {
            Ok(v) => Ok(v),
            Err(AppError::Api { status, .. }) if status == 404 || status == 405 => {
                let existing = self
                    .get(&format!("/api/workItems/{wid}"), &self.fq(F_WORKITEM))
                    .await?;
                let merged = crate::model::WorkitemWrite {
                    op: crate::model::WorkOp::Create,
                    issue_id: issue.clone(),
                    work_item_id: None,
                    date: a.date.clone().or_else(|| {
                        existing
                            .get("date")
                            .and_then(|x| x.as_i64())
                            .map(|ms| self.epoch_ms_to_iso(ms))
                    }),
                    minutes: a.minutes.or_else(|| {
                        existing.get("duration").and_then(|d| d.get("minutes")).and_then(|x| x.as_i64())
                    }),
                    text: a.text.clone(),
                    description: a
                        .description
                        .clone()
                        .or_else(|| existing.get("description").and_then(|x| x.as_str()).map(String::from)),
                    work_type: a.work_type.clone(),
                    markdown: a.markdown,
                    idempotent: None,
                };
                let created = self.workitem_create(&merged).await?;
                self.delete(&format!("/api/issues/{issue}/timeTracking/workItems/{wid}")).await?;
                Ok(created)
            }
            Err(e) => Err(e),
        }
    }

    pub fn epoch_ms_to_iso(&self, ms: i64) -> String {
        use chrono::TimeZone;
        self.cfg
            .timezone
            .timestamp_millis_opt(ms)
            .single()
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
    }

    pub async fn workitem_delete(&self, issue: &str, wid: &str) -> Result<Value> {
        let issue = self.cfg.expand_issue_id(issue);
        self.delete(&format!("/api/issues/{issue}/timeTracking/workItems/{wid}")).await?;
        Ok(json!({"deleted": true, "issueId": issue, "workItemId": wid}))
    }

    // ---- activities ----

    /// Activity time bounds accept ISO YYYY-MM-DD or a raw unix-ms integer.
    fn activity_ts(&self, s: &str) -> Result<i64> {
        match s.parse::<i64>() {
            Ok(n) => Ok(n),
            Err(_) => self.date_to_epoch_ms(s),
        }
    }

    fn activity_categories(cats: Option<&[String]>) -> String {
        match cats {
            Some(c) if !c.is_empty() => c.join(","),
            _ => ACTIVITY_DEFAULT_CATEGORIES.to_string(),
        }
    }

    pub async fn issue_activities(
        &self,
        issue: &str,
        author: Option<&str>,
        start: Option<&str>,
        end: Option<&str>,
        categories: Option<&[String]>,
        top: i64,
        skip: i64,
    ) -> Result<Value> {
        let issue = self.cfg.expand_issue_id(issue);
        let mut q = vec![
            ("fields", F_ACTIVITY.to_string()),
            ("categories", Self::activity_categories(categories)),
            ("$top", top.to_string()),
            ("$skip", skip.to_string()),
        ];
        if let Some(a) = author {
            q.push(("author", self.cfg.resolve_alias(a).to_string()));
        }
        if let Some(s) = start {
            q.push(("start", self.activity_ts(s)?.to_string()));
        }
        if let Some(e) = end {
            q.push(("end", self.activity_ts(e)?.to_string()));
        }
        self.get(&format!("/api/issues/{issue}/activities"), &q).await
    }

    pub async fn users_activity(
        &self,
        author: &str,
        start: Option<&str>,
        end: Option<&str>,
        categories: Option<&[String]>,
        reverse: Option<bool>,
        top: i64,
        skip: i64,
    ) -> Result<Value> {
        let end_ms = match end {
            Some(e) => self.activity_ts(e)?,
            None => chrono::Utc::now().timestamp_millis(),
        };
        let start_ms = match start {
            Some(s) => self.activity_ts(s)?,
            None => end_ms - 30 * 24 * 60 * 60 * 1000,
        };
        if start_ms > end_ms {
            return Err(AppError::Bad("startDate after endDate".into()));
        }
        let mut q = vec![
            ("fields", F_ACTIVITY.to_string()),
            ("author", self.cfg.resolve_alias(author).to_string()),
            ("categories", Self::activity_categories(categories)),
            ("start", start_ms.to_string()),
            ("end", end_ms.to_string()),
            ("$top", top.to_string()),
            ("$skip", skip.to_string()),
        ];
        if let Some(r) = reverse {
            q.push(("reverse", r.to_string()));
        }
        self.get("/api/activities", &q).await
    }

    // ---- users / meta ----

    pub async fn users_list(&self, query: Option<&str>) -> Result<Value> {
        let mut q = vec![("fields", F_USERS.to_string()), ("$top", "100".to_string())];
        if let Some(query) = query {
            q.push(("query", query.to_string()));
        }
        self.get("/api/users", &q).await
    }

    pub async fn user_current(&self) -> Result<Value> {
        self.get("/api/users/me", &self.fq(F_USERS)).await
    }

    async fn current_login(&self) -> Result<String> {
        if let Some(l) = self.current_login.read().await.clone() {
            return Ok(l);
        }
        let login = self
            .user_current()
            .await?
            .get("login")
            .and_then(|x| x.as_str())
            .map(String::from)
            .ok_or_else(|| AppError::Bad("cannot resolve current user".into()))?;
        *self.current_login.write().await = Some(login.clone());
        Ok(login)
    }

    pub async fn user_get(&self, id: &str) -> Result<Value> {
        self.get(&format!("/api/users/{id}"), &self.fq(F_USERS)).await
    }

    pub async fn meta_projects(&self) -> Result<Value> {
        self.get("/api/admin/projects", &[("fields", F_PROJECTS.to_string()), ("$top", "500".to_string())]).await
    }

    pub async fn meta_link_types(&self) -> Result<Value> {
        self.get("/api/issueLinkTypes", &self.fq(F_LINK_TYPES)).await
    }

    pub async fn meta_work_types(&self, project: Option<&str>) -> Result<Value> {
        match project {
            Some(p) => {
                let pid = self.project_id(p).await?;
                self.get(
                    &format!("/api/admin/projects/{pid}/timeTrackingSettings/workItemTypes"),
                    &self.fq("id,name"),
                )
                .await
            }
            None => {
                self.get("/api/admin/timeTrackingSettings/workItemTypes", &self.fq("id,name")).await
            }
        }
    }

    // ---- attachments ----

    pub async fn attachments_list(&self, issue: &str) -> Result<Value> {
        let issue = self.cfg.expand_issue_id(issue);
        self.get(&format!("/api/issues/{issue}/attachments"), &self.fq(F_ATTACH)).await
    }

    pub async fn attachment_get(&self, issue: &str, aid: &str) -> Result<Value> {
        let issue = self.cfg.expand_issue_id(issue);
        self.get(&format!("/api/issues/{issue}/attachments/{aid}"), &self.fq(F_ATTACH)).await
    }

    pub async fn attachment_upload(
        &self,
        issue: &str,
        name: &str,
        bytes: Vec<u8>,
    ) -> Result<Value> {
        let issue = self.cfg.expand_issue_id(issue);
        let part = reqwest::multipart::Part::bytes(bytes).file_name(name.to_string());
        let form = reqwest::multipart::Form::new().part(name.to_string(), part);
        let resp = self
            .http
            .post(self.url(&format!("/api/issues/{issue}/attachments")))
            .query(&[("fields", F_ATTACH)])
            .multipart(form)
            .send()
            .await?;
        self.check(resp).await
    }

    pub async fn attachment_download(&self, issue: &str, aid: &str) -> Result<(String, Vec<u8>)> {
        let meta = self.attachment_get(issue, aid).await?;
        let name = meta
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("attachment")
            .to_string();
        let rel = meta
            .get("url")
            .and_then(|x| x.as_str())
            .ok_or_else(|| AppError::Bad("attachment has no url".into()))?;
        let full = if rel.starts_with("http") {
            rel.to_string()
        } else {
            format!("{}{}", self.cfg.base_url, rel)
        };
        let resp = self.http.get(full).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Api {
                status: resp.status().as_u16(),
                message: "attachment download failed".into(),
            });
        }
        Ok((name, resp.bytes().await?.to_vec()))
    }

    pub async fn attachment_delete(&self, issue: &str, aid: &str) -> Result<Value> {
        let issue = self.cfg.expand_issue_id(issue);
        self.delete(&format!("/api/issues/{issue}/attachments/{aid}")).await?;
        Ok(json!({"deleted": true, "issueId": issue, "attachmentId": aid}))
    }

    pub fn b64_decode(s: &str) -> Result<Vec<u8>> {
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(|e| AppError::Bad(format!("bad base64: {e}")))
    }

    pub fn b64_encode(b: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_detection() {
        assert!(is_id("2-42310"));
        assert!(is_id("0-92"));
        assert!(!is_id("Culture-510"));
        assert!(!is_id("123"));
        assert!(!is_id("a-b"));
    }

    #[test]
    fn b64_roundtrip() {
        let data = b"hello attach";
        let enc = YouTrack::b64_encode(data);
        assert_eq!(YouTrack::b64_decode(&enc).unwrap(), data);
    }
}
