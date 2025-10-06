use k8s_openapi::{
    api::core::v1::{Node, Pod},
    apimachinery::pkg::apis::meta::v1::{Condition, Time},
    chrono::Utc,
};
use kube::Resource;

pub trait HasStatusConditions {
    /// Returns the [`Condition`] with the specified type if it exists or a default with status "Unknown"
    ///
    /// This does not modify the actual list of conditions.
    fn condition(&self, type_: impl ToString + Clone) -> Condition;
    /// Inserts a [`Condition`] with status "Unknown" if no condition with the specified type
    /// exists. Then returns a mutable reference to that [`Condition`]
    fn condition_mut(&mut self, type_: impl ToString + Clone) -> &mut Condition;
}

pub trait ConditionExt {
    /// Returns a value indicating whether the condition has status "True"
    fn is_true(&self) -> bool;
    /// Returns a value indicating whether the condition has status "False"
    fn is_false(&self) -> bool;
    /// Returns a value indicating whether the condition has status "Unknown"
    fn is_unknown(&self) -> bool;
    /// Returns a value indicating whether the condition has the indicated reason.
    fn has_reason(&self, reason: impl ToString) -> bool;
    /// Returns a value indicating whether the condition has the same generation as the resource.
    fn is_current(&self, resource: impl Resource) -> bool;

    /// Sets the status of this condition to "True", updating the lastTransitionTime if necessary.
    fn set_true(&mut self);
    /// Sets the status of this condition to "False", updating the lastTransitionTime if necessary.
    fn set_false(&mut self);
    /// Sets the status of this condition to "Unknown", updating the lastTransitionTime if necessary.
    fn set_unknown(&mut self);
    /// Sets the reason of this condition to the given reason, updating the lastTransitionTime if necessary.
    fn set_reason(&mut self, reason: impl ToString);
    /// Sets the message of this condition to the given message, updating the lastTransitionTime if necessary.
    fn set_message(&mut self, message: impl ToString);
    /// Sets the generation of this condition to the generation of the given resource, updating the lastTransitionTime if necessary.
    fn set_generation_from(&mut self, resource: impl Resource);
}

impl ConditionExt for Condition {
    #[inline]
    fn is_true(&self) -> bool {
        self.status == "True"
    }

    #[inline]
    fn is_false(&self) -> bool {
        self.status == "False"
    }

    #[inline]
    fn is_unknown(&self) -> bool {
        self.status == "Unknown"
    }

    #[inline]
    fn has_reason(&self, reason: impl ToString) -> bool {
        self.reason == reason.to_string()
    }

    #[inline]
    fn is_current(&self, resource: impl Resource) -> bool {
        self.observed_generation == resource.meta().generation
    }

    fn set_true(&mut self) {
        update_condition(self, |c| c.status = "True".to_string());
    }

    fn set_false(&mut self) {
        update_condition(self, |c| c.status = "False".to_string());
    }

    fn set_unknown(&mut self) {
        update_condition(self, |c| c.status = "Unknown".to_string());
    }

    fn set_reason(&mut self, reason: impl ToString) {
        let r = reason.to_string();
        update_condition(self, |c| c.reason = r.clone());
    }

    fn set_message(&mut self, message: impl ToString) {
        let msg = message.to_string();
        update_condition(self, |c| c.message = msg.clone());
    }

    fn set_generation_from(&mut self, resource: impl Resource) {
        update_condition(self, |c| c.observed_generation = resource.meta().generation);
    }
}

fn update_condition<F>(condition: &mut Condition, mut f: F) -> &mut Condition
where
    F: FnMut(&mut Condition),
{
    let mut modified_condition = condition.clone();
    f(&mut modified_condition);
    if modified_condition.observed_generation != condition.observed_generation
        || modified_condition.reason != condition.reason
        || modified_condition.status != condition.status
        || modified_condition.message != condition.message
    {
        f(condition);
        condition.last_transition_time = Time(Utc::now());
    }
    condition
}

fn generate_unknown_condition(type_: String) -> Condition {
    Condition {
        last_transition_time: Time(Utc::now()),
        message: "".to_string(),
        observed_generation: None,
        reason: "".to_string(),
        status: "Unknown".to_string(),
        type_,
    }
}

// Implementations for existing things.
impl HasStatusConditions for Pod {
    fn condition(&self, type_: impl ToString + Clone) -> Condition {
        self.status
            .clone()
            .unwrap_or_default()
            .conditions
            .unwrap_or_default()
            .iter()
            .find(|c| c.type_ == type_.clone().to_string())
            .cloned()
            .map(|pc| Condition {
                last_transition_time: pc.last_transition_time.unwrap_or_else(|| Time(Utc::now())),
                message: pc.message.unwrap_or_default(),
                observed_generation: None,
                reason: pc.reason.unwrap_or_default(),
                status: pc.status,
                type_: pc.type_,
            })
            .unwrap_or_else(|| generate_unknown_condition(type_.to_string()))
    }

    fn condition_mut(&mut self, _type_: impl ToString + Clone) -> &mut Condition {
        // Mutating PodConditions is not supported.
        unimplemented!()
    }
}

impl HasStatusConditions for Node {
    fn condition(&self, type_: impl ToString + Clone) -> Condition {
        self.status
            .clone()
            .unwrap_or_default()
            .conditions
            .unwrap_or_default()
            .iter()
            .find(|c| c.type_ == type_.clone().to_string())
            .cloned()
            .map(|nc| Condition {
                last_transition_time: nc.last_transition_time.unwrap_or_else(|| Time(Utc::now())),
                message: nc.message.unwrap_or_default(),
                observed_generation: None,
                reason: nc.reason.unwrap_or_default(),
                status: nc.status,
                type_: nc.type_,
            })
            .unwrap_or_else(|| generate_unknown_condition(type_.to_string()))
    }

    fn condition_mut(&mut self, _type_: impl ToString + Clone) -> &mut Condition {
        // Mutating NodeConditions is not supported.
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Display;

    use super::*;
    use kube::CustomResource;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(CustomResource, Clone, Debug, Serialize, Deserialize, JsonSchema)]
    #[kube(
        group = "dummy.io",
        version = "v1alpha1",
        kind = "Dummy",
        status = "DummyStatus"
    )]
    struct DummySpec {}

    #[derive(Default, Clone, Debug, Serialize, Deserialize, JsonSchema)]
    struct DummyStatus {
        pub conditions: Option<Vec<Condition>>,
    }

    impl HasStatusConditions for Dummy {
        fn condition(&self, type_: impl ToString + Clone) -> Condition {
            self.status
                .clone()
                .unwrap_or_default()
                .conditions
                .unwrap_or_default()
                .iter()
                .find(|c| c.type_ == type_.clone().to_string())
                .cloned()
                .unwrap_or_else(|| generate_unknown_condition(type_.to_string()))
        }

        fn condition_mut(&mut self, type_: impl ToString + Clone) -> &mut Condition {
            let conditions = self
                .status
                .get_or_insert_default()
                .conditions
                .get_or_insert_default();

            // Fallback to empty condition with status "Unknown"
            let condition = generate_unknown_condition(type_.clone().to_string());
            conditions.push(condition);
            conditions.sort_by_key(|c| c.type_.clone());
            conditions.dedup_by_key(|c| c.type_.clone());

            conditions
                .iter_mut()
                .find(|c| c.type_ == type_.clone().to_string())
                .unwrap()
            // This unwrap is safe. We added the condition we are looking for just a
            // few lines ago. We need to find it again to "play nice" with lifetimes.
        }
    }

    enum ReadyReasons {
        Yoyo,
    }

    impl Display for ReadyReasons {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ReadyReasons::Yoyo => write!(f, "Yoyo"),
            }
        }
    }

    macro_rules! compare_reasons {
        ($($name:ident: $value:expr,)*) => {
            mod compare_reasons {
                use super::*;
                $(
                    #[test]
                    fn $name() {
                        let (input, expected) = $value;
                        let mut dummy = Dummy::new("dummy", DummySpec {});
                        dummy.condition_mut("Ready").set_reason(input);
                        assert!(dummy.condition("Ready").has_reason(expected));
                    }
                )*
            }
        }
    }

    compare_reasons! {
        string_to_str: ("Yoyo".to_string(), "Yoyo"),
        string_to_string: ("Yoyo".to_string(), "Yoyo".to_string()),
        str_to_str: ("Yoyo", "Yoyo"),
        enum_to_str: (ReadyReasons::Yoyo, "Yoyo"),
        enum_to_string: (ReadyReasons::Yoyo, "Yoyo".to_string()),
    }
}
