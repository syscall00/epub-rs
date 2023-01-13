//! Manages the zip component part of the epub doc.
//!
//! Provides easy methods to navigate through the epub parts and to get
//! the content as string.

use anyhow::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use std::io::{Read, Seek};

/// Epub archive struct. Here it's stored the file path and the list of
/// files in the zip archive.
pub struct EpubArchive<R: Read + Seek> {
    zip: zip::ZipArchive<R>,
    pub path: PathBuf,
    pub files: Vec<String>,
}

impl EpubArchive<BufReader<File>> {
    /// Opens the epub file in `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the zip is broken or if the file doesn't
    /// exists.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<EpubArchive<BufReader<File>>, Error> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut archive = EpubArchive::from_reader(BufReader::new(file))?;
        archive.path = path.to_path_buf();
        Ok(archive)
    }
}

impl<R: Read + Seek> EpubArchive<R> {
    /// Opens the epub contained in `reader`.
    ///
    /// # Errors
    ///
    /// Returns an error if the zip is broken.
    pub fn from_reader(reader: R) -> Result<EpubArchive<R>, Error> {
        let zip = zip::ZipArchive::new(reader)?;

        let files:Vec<String> = zip.file_names().map(|f| f.to_string()).collect();

        Ok(EpubArchive {
            zip,
            path: PathBuf::new(),
            files,
        })
    }

    /// Returns the content of the file by the `name` as `Vec<u8>`.
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't exists in the zip archive.
    pub fn get_entry<P: AsRef<Path>>(&mut self, name: P) -> Result<Vec<u8>, Error> {
        let mut entry: Vec<u8> = vec![];
        let name = name.as_ref().display().to_string();
        match self.zip.by_name(&name) {
            Ok(mut zipfile) => {
                zipfile.read_to_end(&mut entry)?;
                return Ok(entry);
            }
            Err(zip::result::ZipError::FileNotFound) => {}
            Err(e) => {
                return Err(e.into());
            }
        };

        // try percent encoding
        let name = percent_encoding::percent_decode(name.as_str().as_bytes()).decode_utf8()?;
        let mut zipfile = self.zip.by_name(&name)?;
        zipfile.read_to_end(&mut entry)?;
        Ok(entry)
    }

    /// Returns the content of the file by the `name` as `String`.
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't exists in the zip archive.
    pub fn get_entry_as_str<P: AsRef<Path>>(&mut self, name: P) -> Result<String, Error> {
        let content = self.get_entry(name)?;
        String::from_utf8(content).map_err(Error::from)
    }

    /// Returns the content of container file "META-INF/container.xml".
    ///
    /// # Errors
    ///
    /// Returns an error if the epub doesn't have the container file.
    pub fn get_container_file(&mut self) -> Result<Vec<u8>, Error> {
        let content = self.get_entry("META-INF/container.xml")?;
        Ok(content)
    }

    /// Modify a resource in the archive and save the archive on the disk.
    /// This method can be called either from EpubArchive or EpubDoc
    /// 
    /// # Errors
    ///
    /// Returns an error if the epub archive does not have the page to modify or if
    /// there is an error during writing of zip file.
    pub fn modify_entry<P: AsRef<Path>>(&mut self, path : &File, page_to_modify : P,  new_content: &str) -> Result<(), Error> {

        let page_to_modify = page_to_modify.as_ref().display().to_string();

        // check if the file exists
        match self.zip.by_name(&page_to_modify) {
            Ok(_) => {}
            Err(zip::result::ZipError::FileNotFound) => {
                return Err(Error::msg("File not found"));
            }
            Err(e) => {
                return Err(e.into());
            }
        };
        
        // create an empty zip archive
        let mut zip_writer = zip::ZipWriter::new(path);

        for i in 0..self.zip.len() {
            let file = self.zip.by_index(i).unwrap();

            
            if file.name() == page_to_modify {
                zip_writer.start_file(file.name(), zip::write::FileOptions::default())?;
                std::io::Write::write_all(&mut zip_writer, new_content.as_bytes())?;
            }
            
            else {
                zip_writer.raw_copy_file(file).unwrap();
            }
        }

        zip_writer.finish().unwrap();
        Ok(())
    }
}
