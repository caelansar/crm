use crate::{
    pb::{WelcomeRequest, WelcomeResponse},
    CrmService,
};
use chrono::{Duration, Utc};
use crm_metadata::pb::{Content, MaterializeRequest};
use crm_notification::pb::SendRequest;
use futures::StreamExt;
use tonic::{Response, Status};
use tracing::debug;
use user_stat::pb::QueryRequest;

impl CrmService {
    pub async fn welcome(&self, req: WelcomeRequest) -> Result<Response<WelcomeResponse>, Status> {
        let request_id = req.id;
        let d1 = Utc::now() - Duration::days(req.interval as _);
        let d2 = d1 + Duration::days(1);
        let query = QueryRequest::new_with_dt("created_at", d1, d2);
        let res_user_stats = self
            .user_stats_pool
            .get()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .query(query)
            .await?
            .into_inner();

        let contents = self
            .metadata_pool
            .get()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .materialize(MaterializeRequest::new_with_ids(&req.content_ids))
            .await?
            .into_inner();

        let contents: Vec<Content> = contents
            .filter_map(|v| async move { v.ok() })
            .collect()
            .await;

        debug!("contents: {:?}", contents);

        let sender = self.config.server.sender_email.clone();
        let reqs = res_user_stats.filter_map(move |v| {
            let sender: String = sender.clone();
            async move {
                let v = v.ok()?;
                debug!("sending email to {}", v.email);
                Some(SendRequest::new_email(
                    "Welcome".to_string(),
                    sender,
                    &[v.email],
                ))
            }
        });

        self.notification_pool
            .get()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .send(reqs)
            .await?;

        Ok(Response::new(WelcomeResponse { id: request_id }))
    }
}
