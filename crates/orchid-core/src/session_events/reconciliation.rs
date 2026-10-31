use super::*;
impl SessionEventApplication {
    pub fn reconcile_steering(&self, session: &ReferenceIdentity) -> Result<(), String> {
        for mut delivery in self
            .store
            .deliveries_for_session(session)
            .map_err(|e| e.to_string())?
        {
            let DeliveryOutcome::Dispatched {
                invocation,
                input_id: Some(input),
                state,
            } = &delivery.outcome
            else {
                continue;
            };
            if matches!(
                state.as_deref(),
                Some("accepted" | "expired" | "rejected" | "superseded")
            ) {
                continue;
            }
            let Some(next) = self
                .dispatcher
                .steering_state(session, invocation, input)
                .map_err(|e| e.to_string())?
            else {
                continue;
            };
            if state.as_deref() == Some(&next) {
                continue;
            }
            if let DeliveryOutcome::Dispatched { state, .. } = &mut delivery.outcome {
                *state = Some(next);
            }
            self.store
                .update_delivery(&delivery)
                .map_err(|e| e.to_string())?;
            if let Some(observer) = &self.record_observer {
                if let Some(group) = self
                    .store
                    .event_group(&delivery.event_group_id)
                    .map_err(|e| e.to_string())?
                {
                    observer(&SessionEventResult {
                        deliveries: self
                            .store
                            .deliveries_for_group(&group.event_group_id)
                            .map_err(|e| e.to_string())?,
                        group,
                    });
                }
            }
        }
        Ok(())
    }
    pub fn recorded_result(
        &self,
        id: &ReferenceIdentity,
    ) -> Result<Option<SessionEventResult>, String> {
        self.store
            .event_group(id)
            .map_err(|e| e.to_string())?
            .map(|group| {
                Ok(SessionEventResult {
                    deliveries: self
                        .store
                        .deliveries_for_group(id)
                        .map_err(|e| e.to_string())?,
                    group,
                })
            })
            .transpose()
    }
    pub fn retry_expired_delivery(
        &self,
        id: &ReferenceIdentity,
        allow: bool,
    ) -> Result<Option<SessionEventResult>, String> {
        let Ok(_lane) = self.retry_lane.try_lock() else {
            return Ok(None);
        };
        let Some(mut result) = self.recorded_result(id)? else {
            return Ok(None);
        };
        let mut changed = false;
        for delivery in &mut result.deliveries {
            if !matches!(&delivery.outcome,DeliveryOutcome::Dispatched {input_id:Some(_),state:Some(state),..} if state=="expired" || state=="rejected")
            {
                continue;
            }
            if !allow {
                if let DeliveryOutcome::Dispatched { state, .. } = &mut delivery.outcome {
                    *state = Some("superseded".into());
                }
            } else {
                let receipt = self
                    .dispatcher
                    .dispatch(SessionInvocationRequest {
                        delivery_mode: SessionDeliveryMode::Queue,
                        event_group_id: result.group.event_group_id.clone(),
                        delivery_id: delivery.delivery_id.clone(),
                        target_session: delivery.target_session.clone(),
                        source: result.group.source.clone(),
                        prompt_sources: delivery.prompt_contributions.clone(),
                        initial_prompt_sources: vec![],
                        direct_user_options: None,
                    })
                    .map_err(|e| e.to_string())?;
                delivery.outcome = DeliveryOutcome::Dispatched {
                    invocation: receipt.invocation,
                    input_id: receipt.input_id,
                    state: receipt.state,
                };
            }
            self.store
                .update_delivery(delivery)
                .map_err(|e| e.to_string())?;
            changed = true;
        }
        if changed {
            if let Some(observer) = &self.record_observer {
                observer(&result);
            }
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}
