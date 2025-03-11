pub use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;
use sweet::prelude::FsExt;
use sweet::prelude::ReadDir;
use sweet::prelude::ReadFile;
use syn::spanned::Spanned;
use syn::visit::Visit;
mod hash_file;
mod template_watcher;
pub use hash_file::*;
pub use template_watcher::*;


#[derive(Debug)]
pub struct BuildRsxTemplateMap {
	/// use [ron::ser::to_string_pretty] instead of
	/// directly serializing the ron tokens.
	pub pretty: bool,
	pub src: PathBuf,
	// keep default in sync with StaticFileRouter
	pub dst: PathBuf,
}

impl BuildRsxTemplateMap {
	pub const DEFAULT_TEMPLATES_DST: &'static str = "target/rsx-templates.ron";

	pub fn new(src: impl Into<PathBuf>) -> Self {
		Self::new_with_dst(src, Self::DEFAULT_TEMPLATES_DST)
	}
	pub fn new_with_dst(
		src: impl Into<PathBuf>,
		dst: impl Into<PathBuf>,
	) -> Self {
		Self {
			pretty: true,
			src: src.into(),
			dst: dst.into(),
		}
	}

	pub fn build_and_write(&self) -> Result<()> {
		let map_tokens = self.build_ron()?;
		let mut map_str = map_tokens.to_string();
		if self.pretty {
			let map = ron::de::from_str::<RsxTemplateMap>(&map_str)?;
			map_str = ron::ser::to_string_pretty(&map, Default::default())?;
		}
		FsExt::write(&self.dst, &map_str)?;
		Ok(())
	}


	pub fn build_ron(&self) -> Result<TokenStream> {
		let items = ReadDir::files_recursive(&self.src)?
			.into_iter()
			.map(|path| self.file_templates(path))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.map(|(RsxMacroLocation { file, line, col }, tokens)| {
				let line = Literal::usize_unsuffixed(line);
				let col = Literal::usize_unsuffixed(col);
				quote! {
					RsxMacroLocation(
						file: #file,
						line: #line,
						col: #col
					):#tokens
				}
			});

		let map = quote! {
			RsxTemplateMap({#(#items),*})
		};
		Ok(map)
	}

	/// get all the rsx macros in this file.
	/// If it doesnt have a rust extension an empty vec is returned
	fn file_templates(
		&self,
		path: PathBuf,
	) -> Result<Vec<(RsxMacroLocation, TokenStream)>> {
		if path.extension().map_or(false, |ext| ext == "rs") {
			let file = ReadFile::to_string(&path)?;
			let file = syn::parse_file(&file)?;
			let mac = syn::parse_quote!(rsx);
			let mut visitor = RsxSynVisitor::new(path.to_string_lossy(), mac);

			visitor.visit_file(&file);
			Ok(visitor.templates)
		} else {
			Ok(Default::default())
		}
	}
}

#[derive(Debug)]
struct RsxSynVisitor {
	file: String,
	templates: Vec<(RsxMacroLocation, TokenStream)>,
	mac: syn::Ident,
}
impl RsxSynVisitor {
	pub fn new(file: impl Into<String>, mac: syn::Ident) -> Self {
		Self {
			file: file.into(),
			templates: Default::default(),
			mac,
		}
	}
}


impl<'a> Visit<'a> for RsxSynVisitor {
	fn visit_macro(&mut self, mac: &syn::Macro) {
		if mac
			.path
			.segments
			.last()
			.map_or(false, |seg| seg.ident == self.mac)
		{
			// use the span of the inner tokens to match the behavior of
			// the rsx! macro
			let span = mac.tokens.span();
			let start = span.start();
			let loc =
				RsxMacroLocation::new(&self.file, start.line, start.column);
			let tokens = RstmlToRsxTemplate::default()
				.map_tokens(mac.tokens.clone(), &self.file);
			self.templates.push((loc, tokens));
		}
	}
}


#[cfg(test)]
mod test {
	use std::path::PathBuf;

	use crate::prelude::*;
	use beet_rsx::rsx::RsxTemplateMap;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let src =
			FsExt::workspace_root().join("crates/beet_router/src/test_site");

		let file = BuildRsxTemplateMap {
			src,
			dst: PathBuf::default(),
			pretty: true,
		}
		.build_ron()
		.unwrap()
		.to_string();
		let map: RsxTemplateMap = ron::de::from_str(&file).unwrap();
		expect(map.len()).to_be(4);
		// println!("{:#?}", map);
	}
}
