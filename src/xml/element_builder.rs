// RustyXML
// Copyright (c) 2013, 2014 Florian Zeitz
//
// This project is MIT licensed.
// Please see the COPYING file for more information.

use super::{Event, Xml, Element, StartTag, EndTag};
use parser::ParserError;
use std::borrow::ToOwned;
use std::collections::HashMap;
use std::error::{Error, FromError};
use std::fmt;

#[derive(PartialEq, Debug, Clone)]
/// The structure returned for errors encountered while building an `Element`
pub enum BuilderError {
    /// Errors encountered by the `Parser`
    Parser(ParserError),
    /// Elements were improperly nested, e.g. <a><b></a></b>
    ImproperNesting,
    /// No element was found
    NoElement
}

impl Error for BuilderError {
    fn description(&self) -> &str {
        match *self {
            BuilderError::Parser(ref err) => err.description(),
            BuilderError::ImproperNesting => "Elements not properly nested",
            BuilderError::NoElement => "No elements found"
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            BuilderError::Parser(ref err) => Some(err),
            _ => None
        }
    }
}

impl fmt::Display for BuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BuilderError::Parser(ref err) => err.fmt(f),
            BuilderError::ImproperNesting => write!(f, "Elements not properly nested"),
            BuilderError::NoElement => write!(f, "No elements found")
        }
    }
}

impl FromError<ParserError> for BuilderError {
    fn from_error(err: ParserError) -> BuilderError { BuilderError::Parser(err) }
}

// DOM Builder
/// An Element Builder, building `Element`s from `Event`s as produced by `Parser`
pub struct ElementBuilder {
    stack: Vec<Element>,
    default_ns: Vec<Option<String>>,
    prefixes: HashMap<String, String>
}

impl ElementBuilder {
    /// Returns a new `ElementBuilder`
    pub fn new() -> ElementBuilder {
        let mut prefixes = HashMap::with_capacity(2);
        prefixes.insert("http://www.w3.org/XML/1998/namespace".to_owned(), "xml".to_owned());
        prefixes.insert("http://www.w3.org/2000/xmlns/".to_owned(), "xmlns".to_owned());
        ElementBuilder {
            stack: Vec::new(),
            default_ns: Vec::new(),
            prefixes: prefixes
        }
    }

    /// Bind a prefix to a namespace
    pub fn define_prefix(&mut self, prefix: &str, ns: &str) {
        self.prefixes.insert(ns.to_owned(), prefix.to_owned());
    }

    /// Set the default namespace
    pub fn set_default_ns(&mut self, ns: &str) {
        self.default_ns = vec![Some(ns.to_owned())];
    }

    /// Hands an `Event` to the builder.
    /// While no root element has been finished `Ok(None)` is returned.
    /// Once sufficent data has been received an `Element` is returned as `Ok(elem)`.
    /// Upon Error `Err("message")` is returned.
    pub fn push_event(&mut self,
                      e: Result<Event, ParserError>) -> Result<Option<Element>, BuilderError> {
        let e = try!(e);
        match e {
            Event::PI(cont) => {
                if let Some(elem) = self.stack.last_mut() {
                    elem.children.push(Xml::PINode(cont));
                }
            }
            Event::ElementStart(StartTag { name, ns, prefix: _, attributes }) => {
                let mut elem = Element {
                    name: name.clone(),
                    ns: ns.clone(),
                    default_ns: None,
                    prefixes: self.prefixes.clone(),
                    attributes: attributes,
                    children: Vec::new()
                };

                if !self.default_ns.is_empty() {
                    let cur_default = self.default_ns.last().unwrap().clone();
                    self.default_ns.push(cur_default);
                }

                for (&(ref name, ref ns), value) in elem.attributes.iter() {
                    if ns.is_none() && *name == "xmlns" {
                        self.default_ns.pop();
                        if value.len() == 0 {
                            self.default_ns.push(None);
                        } else {
                            self.default_ns.push(Some(value.clone()));
                        }
                        continue;
                    }

                    if ns.as_ref().map_or(false, |x| *x == "http://www.w3.org/2000/xmlns/") {
                        elem.prefixes.insert(value.clone(), name.clone());
                    }
                }
                elem.default_ns = self.default_ns.last().unwrap_or(&None).clone();

                self.stack.push(elem);
            }
            Event::ElementEnd(EndTag { name, ns, prefix: _ }) => {
                let elem = match self.stack.pop() {
                    Some(elem) => elem,
                    None => return Err(BuilderError::ImproperNesting)
                };
                self.default_ns.pop();
                if elem.name != name || elem.ns != ns {
                    return Err(BuilderError::ImproperNesting)
                } else {
                    match self.stack.last_mut() {
                        Some(e) => e.children.push(Xml::ElementNode(elem)),
                        None => return Ok(Some(elem))
                    }
                }
            }
            Event::Characters(chars) => {
                if let Some(elem) = self.stack.last_mut() {
                    elem.children.push(Xml::CharacterNode(chars));
                }
            }
            Event::CDATA(chars) => {
                if let Some(elem) = self.stack.last_mut() {
                    elem.children.push(Xml::CDATANode(chars));
                }
            }
            Event::Comment(cont) => {
                if let Some(elem) = self.stack.last_mut() {
                    elem.children.push(Xml::CommentNode(cont));
                }
            }
        }
        Ok(None)
    }
}
