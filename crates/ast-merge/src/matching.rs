use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeSignature(pub Vec<String>);

impl NodeSignature {
    pub fn owner_path(path: impl Into<String>) -> Self {
        Self(vec!["owner_path".to_string(), path.into()])
    }

    pub fn content_identity(content: impl AsRef<str>) -> Option<Self> {
        let normalized = content.as_ref().trim();
        if normalized.is_empty() {
            return None;
        }
        Some(Self(vec!["content_identity".to_string(), normalized.to_string()]))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIdentity {
    pub primary: Option<NodeSignature>,
    pub aliases: Vec<NodeSignature>,
}

impl NodeIdentity {
    pub fn new(primary: Option<NodeSignature>) -> Self {
        Self { primary, aliases: Vec::new() }
    }

    pub fn with_alias(mut self, alias: Option<NodeSignature>) -> Self {
        if let Some(alias) = alias
            && self.primary.as_ref() != Some(&alias)
            && !self.aliases.contains(&alias)
        {
            self.aliases.push(alias);
        }
        self
    }

    fn signatures(&self) -> impl Iterator<Item = &NodeSignature> {
        self.primary.iter().chain(&self.aliases)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIdentityMatch {
    pub template_index: usize,
    pub destination_index: usize,
    pub signature: NodeSignature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIdentityMatchResult {
    pub matched: Vec<NodeIdentityMatch>,
    pub unmatched_template: Vec<usize>,
    pub unmatched_destination: Vec<usize>,
}

pub fn match_node_identities(
    template: &[NodeIdentity],
    destination: &[NodeIdentity],
) -> NodeIdentityMatchResult {
    let mut matched_destination = HashSet::new();
    let mut matched_template = HashSet::new();
    let mut matched = match_identity_pass(
        template,
        destination,
        &mut matched_template,
        &mut matched_destination,
        |identity| identity.primary.iter(),
    );
    matched.extend(match_identity_pass(
        template,
        destination,
        &mut matched_template,
        &mut matched_destination,
        NodeIdentity::signatures,
    ));
    matched.sort_by_key(|entry| entry.template_index);

    let unmatched_template =
        (0..template.len()).filter(|index| !matched_template.contains(index)).collect();
    let unmatched_destination =
        (0..destination.len()).filter(|index| !matched_destination.contains(index)).collect();
    NodeIdentityMatchResult { matched, unmatched_template, unmatched_destination }
}

fn match_identity_pass<'a, I>(
    template: &'a [NodeIdentity],
    destination: &'a [NodeIdentity],
    matched_template: &mut HashSet<usize>,
    matched_destination: &mut HashSet<usize>,
    signatures: impl Fn(&'a NodeIdentity) -> I,
) -> Vec<NodeIdentityMatch>
where
    I: Iterator<Item = &'a NodeSignature>,
{
    let mut destination_by_signature: HashMap<NodeSignature, VecDeque<usize>> = HashMap::new();
    for (index, identity) in destination.iter().enumerate() {
        if matched_destination.contains(&index) {
            continue;
        }
        for signature in signatures(identity) {
            destination_by_signature.entry(signature.clone()).or_default().push_back(index);
        }
    }

    let mut matched = Vec::new();
    for (template_index, identity) in template.iter().enumerate() {
        if matched_template.contains(&template_index) {
            continue;
        }
        let selected = signatures(identity).find_map(|signature| {
            let candidates = destination_by_signature.get_mut(signature)?;
            while let Some(destination_index) = candidates.pop_front() {
                if !matched_destination.contains(&destination_index) {
                    return Some((destination_index, signature.clone()));
                }
            }
            None
        });
        if let Some((destination_index, signature)) = selected {
            matched_template.insert(template_index);
            matched_destination.insert(destination_index);
            matched.push(NodeIdentityMatch { template_index, destination_index, signature });
        }
    }
    matched
}

pub fn match_owner_paths<T, D>(
    template: &[T],
    destination: &[D],
    template_path: impl for<'a> Fn(&'a T) -> &'a str,
    destination_path: impl for<'a> Fn(&'a D) -> &'a str,
) -> NodeIdentityMatchResult {
    let template = template
        .iter()
        .map(|owner| NodeIdentity::new(Some(NodeSignature::owner_path(template_path(owner)))))
        .collect::<Vec<_>>();
    let destination = destination
        .iter()
        .map(|owner| NodeIdentity::new(Some(NodeSignature::owner_path(destination_path(owner)))))
        .collect::<Vec<_>>();
    match_node_identities(&template, &destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_duplicate_signatures_by_occurrence_without_reusing_nodes() {
        let signature = || NodeIdentity::new(Some(NodeSignature::owner_path("/same")));
        let result = match_node_identities(
            &[signature(), signature(), NodeIdentity::new(Some(NodeSignature::owner_path("/new")))],
            &[signature(), signature(), NodeIdentity::new(Some(NodeSignature::owner_path("/old")))],
        );

        assert_eq!(
            result
                .matched
                .iter()
                .map(|entry| (entry.template_index, entry.destination_index))
                .collect::<Vec<_>>(),
            [(0, 0), (1, 1)]
        );
        assert_eq!(result.unmatched_template, [2]);
        assert_eq!(result.unmatched_destination, [2]);
    }

    #[test]
    fn uses_content_identity_only_when_primary_signatures_do_not_match() {
        let shared_content = NodeSignature::content_identity("same body");
        let template = NodeIdentity::new(Some(NodeSignature::owner_path("/old")))
            .with_alias(shared_content.clone());
        let destination =
            NodeIdentity::new(Some(NodeSignature::owner_path("/new"))).with_alias(shared_content);
        let result = match_node_identities(&[template], &[destination]);

        assert_eq!(result.matched.len(), 1);
        assert_eq!(result.matched[0].signature.0[0], "content_identity");
    }

    #[test]
    fn fallback_aliases_cannot_steal_a_later_primary_match() {
        let template = [
            NodeIdentity::new(Some(NodeSignature::owner_path("/deleted")))
                .with_alias(Some(NodeSignature::owner_path("/position/0"))),
            NodeIdentity::new(Some(NodeSignature::owner_path("/retained")))
                .with_alias(Some(NodeSignature::owner_path("/position/1"))),
        ];
        let destination = [NodeIdentity::new(Some(NodeSignature::owner_path("/retained")))
            .with_alias(Some(NodeSignature::owner_path("/position/0")))];
        let result = match_node_identities(&template, &destination);

        assert_eq!(
            result
                .matched
                .iter()
                .map(|entry| (entry.template_index, entry.destination_index))
                .collect::<Vec<_>>(),
            [(1, 0)]
        );
        assert_eq!(result.unmatched_template, [0]);
    }
}
