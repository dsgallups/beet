use crate::prelude::*;

pub type RustyIdx = u32;

///	This struct is the binding between a [RsxNode] and an [HtmlNode].
///
/// Hydrating elements is relatively simple, we can just slap an id on them,
/// but text nodes don't have ids, and to make things even more exciting adjacent
/// nodes are collapsed when rendered.
///
/// ## Footgun
/// These indices are *uncollapsed* indices.
/// When we render html adjacent text nodes are collapsed into a single text node.
/// We use [TextBlockEncoder] to track this behavior, and re-expand the text nodes
/// before using this location.
///
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeLocation {
	/// Incremented every time an rsx node is encountered,
	/// used for reconciliation with the [TreeLocationMap::rusty_locations].
	/// It is required because not all rsx nodes are html nodes.
	pub tree_idx: TreeIdx,
	/// the index of this node's parent *element*. This is used by
	/// text nodes to determine their location in the dom.
	pub parent_idx: TreeIdx,
	/// The *uncollapsed* child index of this node, for
	/// example the following has two child nodes, indexed
	/// as 0 and 1. When it is rendered they will be collapsed
	/// into a single text node, but we will split them up before
	/// using this index via the [TextBlockEncoder].
	///
	/// `<div> hello {"world"}</div>`
	pub child_idx: u32,
	// _padding: u32,
}

impl TreeLocation {
	pub fn new(
		tree_idx: impl Into<TreeIdx>,
		parent_idx: impl Into<TreeIdx>,
		child_idx: u32,
	) -> Self {
		Self {
			tree_idx: tree_idx.into(),
			parent_idx: parent_idx.into(),
			child_idx,
		}
	}

	pub fn to_csv(&self) -> String {
		// must keep in sync with from_csv
		vec![
			self.tree_idx.to_string(),
			self.parent_idx.to_string(),
			self.child_idx.to_string(),
		]
		.join(",")
	}
	pub fn from_csv(csv: &str) -> ParseResult<Self> {
		let mut parts = csv.split(',');
		let mut next = || -> Result<u32, ParseError> {
			let next = parts
				.next()
				.ok_or_else(|| ParseError::serde("invalid rsx context csv"))?
				.parse()?;
			Ok(next)
		};

		// must keep in sync with to_csv
		let tree_idx = next()?;
		let parent_idx = next()?;
		let child_idx = next()?;

		Ok(Self {
			tree_idx: TreeIdx::new(tree_idx),
			parent_idx: TreeIdx::new(parent_idx),
			child_idx,
		})
	}
}



/// Wrapper of a visitor but
#[derive(Debug)]
pub struct TreeLocationVisitor<Func> {
	/// Used by [`TreeLocation::parent_idx`].
	/// we use a stack because [RsxVisitor] is depth-first.
	/// This stack is a breadcrumb trail of parents
	parent_idxs: Vec<TreeIdx>,
	/// Used by [`TreeLocation::child_idx`].
	/// pushed when visiting children, incremented after visiting dom node
	child_idxs: Vec<u32>,
	/// Used by [`TreeLocation::tree_idx`].
	/// Simple counter that increments on each node visited.
	tree_idx_incr: u32,
	options: VisitRsxOptions,
	func: Func,
}
impl<Func> TreeLocationVisitor<Func> {
	/// Visit a node and return the total number of elements visited
	pub fn visit(node: &RsxNode, func: Func)
	where
		Func: FnMut(TreeLocation, &RsxNode),
	{
		Self {
			parent_idxs: vec![Default::default()],
			child_idxs: vec![Default::default()],
			tree_idx_incr: 0,
			options: Default::default(),
			func,
		}
		.walk_node(node);
	}

	pub fn visit_with_options(
		node: &RsxNode,
		options: VisitRsxOptions,
		func: Func,
	) where
		Func: FnMut(TreeLocation, &RsxNode),
	{
		Self {
			parent_idxs: vec![Default::default()],
			child_idxs: vec![Default::default()],
			tree_idx_incr: 0,
			options,
			func,
		}
		.walk_node(node);
	}
	pub fn visit_mut(node: &mut RsxNode, func: Func)
	where
		Func: FnMut(TreeLocation, &mut RsxNode),
	{
		Self {
			parent_idxs: vec![Default::default()],
			child_idxs: vec![Default::default()],
			tree_idx_incr: 0,
			options: Default::default(),
			func,
		}
		.walk_node(node);
	}
	pub fn visit_with_options_mut(
		node: &mut RsxNode,
		options: VisitRsxOptions,
		func: Func,
	) where
		Func: FnMut(TreeLocation, &mut RsxNode),
	{
		Self {
			parent_idxs: Default::default(),
			child_idxs: Default::default(),
			tree_idx_incr: 0,
			options,
			func,
		}
		.walk_node(node);
	}

	/// Get the current item in the stack, or default
	/// # Panics
	/// Panics if the stack is empty
	// pub fn parent(&mut self) -> &mut TreeLocation {
	// 	self.parents
	// 		.last_mut()
	// 		.expect("TreeLocationVisitor stack is empty")
	// }

	pub fn current_location(&self) -> TreeLocation {
		let parent_idx = self.parent_idxs.last().cloned().unwrap_or_default();
		let child_idx = self.child_idxs.last().cloned().unwrap_or_default();
		TreeLocation::new(self.tree_idx_incr, parent_idx, child_idx)
	}
	pub fn after_node(&mut self, node: &RsxNode) {
		self.tree_idx_incr += 1;
		if node.is_html_node() {
			if let Some(child_idx) = self.child_idxs.last_mut() {
				*child_idx += 1;
			}
		}
	}

	pub fn before_children(&mut self) {
		// the reason why we can get the parent idx is because this is called directly after
		// visit_node in RsxVisitor. It also means we can safely decrement by 1 to get
		// the parent index
		self.parent_idxs.push(TreeIdx::new(self.tree_idx_incr - 1));
		self.child_idxs.push(0);
	}
	pub fn after_children(&mut self) {
		self.parent_idxs.pop();
		self.child_idxs.pop();
	}
}


impl<Func: FnMut(TreeLocation, &RsxNode)> RsxVisitor
	for TreeLocationVisitor<Func>
{
	fn options(&self) -> &VisitRsxOptions { &self.options }
	fn visit_node(&mut self, node: &RsxNode) {
		let loc = self.current_location();
		(self.func)(loc, node);
		self.after_node(node);
	}
	fn before_element_children(&mut self, _: &RsxElement) {
		self.before_children();
	}
	fn after_element_children(&mut self, _: &RsxElement) {
		self.after_children();
	}
}
impl<Func: FnMut(TreeLocation, &mut RsxNode)> RsxVisitorMut
	for TreeLocationVisitor<Func>
{
	fn options(&self) -> &VisitRsxOptions { &self.options }
	fn visit_node(&mut self, node: &mut RsxNode) {
		let loc = self.current_location();
		(self.func)(loc, node);
		self.after_node(node);
	}
	fn before_element_children(&mut self, _: &mut RsxElement) {
		self.before_children();
	}
	fn after_element_children(&mut self, _: &mut RsxElement) {
		self.after_children();
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn csv() {
		let a = TreeLocation::new(4, 2, 3);
		let csv = a.to_csv();
		let b = TreeLocation::from_csv(&csv).unwrap();
		expect(a).to_be(b);
	}
	#[test]
	fn works() {
		let bucket = mock_bucket();
		let bucket2 = bucket.clone();
		let rsx = rsx! {
			// 0 - root
			<div>
				// 1 - child
				<div>
					// 2 - nested child
					<div />
					// 3 - second child
					<div />
				</div>
				// 4 - child 1
				<div />
			</div>
		};
		TreeLocationVisitor::visit(&rsx, move |loc, node| {
			if let RsxNode::Element(_) = node {
				bucket2.call(loc);
			}
		});
		expect(&bucket).to_have_been_called_times(5);
		// keep in mind that fragments will also increment
		// the tree_idx..
		expect(&bucket)
			.to_have_returned_nth_with(0, &TreeLocation::new(0, 0, 0));
		expect(&bucket)
			.to_have_returned_nth_with(1, &TreeLocation::new(2, 0, 0));
		expect(&bucket)
			.to_have_returned_nth_with(2, &TreeLocation::new(4, 2, 0));
		expect(&bucket)
			.to_have_returned_nth_with(3, &TreeLocation::new(6, 2, 1));
		expect(&bucket)
			.to_have_returned_nth_with(4, &TreeLocation::new(8, 0, 1));
	}
}
