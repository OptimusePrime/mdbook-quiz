use std::path::Path;

use anyhow::Result;
use mdbook::{
  book::{Book, Chapter},
  preprocess::{Preprocessor, PreprocessorContext},
  BookItem,
};
use pulldown_cmark::{CowStr, Event, Parser};
use pulldown_cmark_to_cmark::cmark;
use regex::Regex;

pub struct QuizProcessor;

pub struct QuizConfig {
  log_endpoint: Option<String>,
  fullscreen: Option<bool>,
}

lazy_static::lazy_static! {
  static ref QUIZ_REGEX: Regex = Regex::new(r"^\{\{#quiz ([^}]+)\}\}$").unwrap();
}

impl QuizProcessor {
  pub fn new() -> Self {
    QuizProcessor
  }

  fn process_quiz(
    &self,
    config: &QuizConfig,
    chapter_dir: &Path,
    quiz_path: &str,
  ) -> Result<String> {
    let quiz_path_rel = Path::new(quiz_path);
    let quiz_path_abs = chapter_dir.join(quiz_path_rel);

    let quiz_name = quiz_path_rel.file_stem().unwrap().to_string_lossy();

    let content_toml = std::fs::read_to_string(quiz_path_abs)?;
    let content = content_toml.parse::<toml::Value>()?;
    let content_json = serde_json::to_string(&content)?;

    let mut html = String::from("<div class=\"quiz-placeholder\"");

    let mut add_data = |k: &str, v: &str| {
      html.push_str(&format!(
        " data-{}=\"{}\" ",
        k,
        html_escape::encode_double_quoted_attribute(v)
      ));
    };
    add_data("quiz-name", &quiz_name);
    add_data("quiz-questions", &content_json);
    if let Some(log_endpoint) = &config.log_endpoint {
      add_data("quiz-log-endpoint", log_endpoint);
    }
    if config.fullscreen.is_some() {
      add_data("quiz-fullscreen", "");
    }

    html.push_str("></div>");

    Ok(html)
  }

  fn process_chapter(
    &self,
    config: &QuizConfig,
    ctx: &PreprocessorContext,
    chapter: &mut Chapter,
  ) -> Result<()> {
    let events = Parser::new(&chapter.content);

    let chapter_path = ctx
      .root
      .join(&ctx.config.book.src)
      .join(chapter.path.as_ref().unwrap());
    let chapter_dir = chapter_path.parent().unwrap();

    let mut new_events = Vec::new();
    for event in events {
      let new_event = match &event {
        Event::Text(text) => {
          let text = text.as_ref();
          match QUIZ_REGEX.captures(text) {
            Some(captures) => {
              let quiz_path = captures.get(1).unwrap().as_str();
              let html = self.process_quiz(config, chapter_dir, quiz_path)?;
              Event::Html(CowStr::Boxed(html.into_boxed_str()))
            }
            None => event,
          }
        }
        _ => event,
      };
      new_events.push(new_event);
    }

    let mut new_content = String::new();
    cmark(new_events.into_iter(), &mut new_content)?;
    chapter.content = new_content;

    Ok(())
  }
}

impl Preprocessor for QuizProcessor {
  fn name(&self) -> &str {
    "quiz"
  }

  fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
    let config_toml = ctx.config.get_preprocessor(self.name()).unwrap();
    let config = QuizConfig {
      log_endpoint: config_toml
        .get("log-endpoint")
        .map(|value| value.as_str().unwrap().to_owned()),
      fullscreen: config_toml
        .get("fullscreen")
        .map(|value| value.as_bool().unwrap()),
    };

    book.for_each_mut(|item| {
      if let BookItem::Chapter(chapter) = item {
        self.process_chapter(&config, ctx, chapter).unwrap();
      }
    });

    Ok(book)
  }

  fn supports_renderer(&self, renderer: &str) -> bool {
    renderer != "not-supported"
  }
}
