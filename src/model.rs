use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteOp {
    Create,
    Update,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Entity {
    Issue,
    Article,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IssueOp {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IssueWrite {
    pub op: IssueOp,
    /// Issue id (required for update; bare number expanded via default project).
    #[serde(default)]
    pub id: Option<String>,
    /// Project shortName or id (required for create).
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub markdown: Option<bool>,
    /// Parent issue id for native subtask hierarchy. Empty string clears it.
    #[serde(default, rename = "parentId")]
    pub parent_id: Option<String>,
    /// Assignee login.
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub state: Option<String>,
    /// Agile board name or id (any language/casing).
    #[serde(default)]
    pub board: Option<String>,
    /// Sprint name or id within `board`. The valid set is board-specific;
    /// an unknown sprint errors with that board's available sprints.
    #[serde(default)]
    pub sprint: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CommentWrite {
    pub entity: Entity,
    pub op: WriteOp,
    /// Issue or article id.
    #[serde(rename = "parentId")]
    pub parent_id: String,
    #[serde(default, rename = "commentId")]
    pub comment_id: Option<String>,
    pub text: String,
    #[serde(default)]
    pub markdown: Option<bool>,
    #[serde(default)]
    pub mute: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ArticleWrite {
    pub op: WriteOp,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default, rename = "parentArticleId")]
    pub parent_article_id: Option<String>,
    #[serde(default)]
    pub markdown: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkOp {
    Add,
    Remove,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkRole {
    Outward,
    Inward,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkWrite {
    pub op: LinkOp,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "targetId")]
    pub target_id: String,
    /// Link type name, e.g. "Relates", "Depend". Not for parent/child — use issue_write.parentId.
    #[serde(rename = "linkType")]
    pub link_type: String,
    /// Semantic direction for directed types. Default outward.
    #[serde(default)]
    pub role: Option<LinkRole>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkOp {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkitemWrite {
    pub op: WorkOp,
    #[serde(rename = "issueId")]
    pub issue_id: String,
    #[serde(default, rename = "workItemId")]
    pub work_item_id: Option<String>,
    /// ISO date YYYY-MM-DD.
    #[serde(default)]
    pub date: Option<String>,
    pub minutes: Option<i64>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Work item type name or id (e.g. "Разработка").
    #[serde(default, rename = "type")]
    pub work_type: Option<String>,
    #[serde(default)]
    pub markdown: Option<bool>,
    /// On create: skip if same issue+date+description already logged.
    #[serde(default)]
    pub idempotent: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdArg {
    pub id: String,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchFields {
    Short,
    Full,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IssueSearch {
    pub query: String,
    #[serde(default)]
    pub fields: Option<SearchFields>,
    #[serde(default)]
    pub top: Option<i64>,
    #[serde(default)]
    pub skip: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CommentsList {
    pub entity: Entity,
    #[serde(rename = "parentId")]
    pub parent_id: String,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GetOp {
    Get,
    List,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ArticleGet {
    pub op: GetOp,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkitemsList {
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default, rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(default, rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(default, rename = "issueId")]
    pub issue_id: Option<String>,
    #[serde(default)]
    pub top: Option<i64>,
    #[serde(default)]
    pub skip: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkitemsReport {
    #[serde(default)]
    pub author: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsersOp {
    List,
    Me,
    Get,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UsersArg {
    pub op: UsersOp,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetaKind {
    Projects,
    LinkTypes,
    WorkItemTypes,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MetaArg {
    pub kind: MetaKind,
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityScope {
    /// Change-history of one issue (GET /api/issues/{id}/activities).
    Issue,
    /// Cross-issue feed for one author (GET /api/activities).
    User,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ActivityArg {
    pub scope: ActivityScope,
    /// Required for scope=issue.
    #[serde(default, rename = "issueId")]
    pub issue_id: Option<String>,
    /// Author login. Required for scope=user; optional filter for scope=issue.
    #[serde(default)]
    pub author: Option<String>,
    /// ISO YYYY-MM-DD or unix ms. scope=user defaults to 30 days back.
    #[serde(default, rename = "startDate")]
    pub start_date: Option<String>,
    /// ISO YYYY-MM-DD or unix ms. Defaults to now.
    #[serde(default, rename = "endDate")]
    pub end_date: Option<String>,
    /// Activity categories. Default CustomFieldCategory,CommentsCategory.
    /// Others: AttachmentsCategory, LinksCategory, WorkItemsActivityCategory,
    /// VcsChangeActivityCategory, TagsCategory, SprintCategory.
    #[serde(default)]
    pub categories: Option<Vec<String>>,
    /// scope=user only: oldest-first when true.
    #[serde(default)]
    pub reverse: Option<bool>,
    #[serde(default)]
    pub top: Option<i64>,
    #[serde(default)]
    pub skip: Option<i64>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttachOp {
    List,
    Get,
    Upload,
    Download,
    Delete,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AttachmentArg {
    pub op: AttachOp,
    #[serde(rename = "issueId")]
    pub issue_id: String,
    #[serde(default, rename = "attachmentId")]
    pub attachment_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "contentBase64")]
    pub content_base64: Option<String>,
    /// Local source for upload, or target for download.
    #[serde(default)]
    pub path: Option<String>,
}
