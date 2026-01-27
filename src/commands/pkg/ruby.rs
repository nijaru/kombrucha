use std::path::Path;
use crate::error::{BruError, Result};
use magnus::{Ruby, Class, value::ReprValue};
use colored::Colorize;

/// Loader for Homebrew Formula DSL via embedded Ruby
pub struct FormulaLoader<'a> {
    ruby: &'a Ruby,
}

impl<'a> FormulaLoader<'a> {
    pub fn new(ruby: &'a Ruby) -> Self {
        Self { ruby }
    }

    /// Setup the basic Ruby environment for Homebrew formulae
    pub fn setup_environment(&self) -> Result<()> {
        // Define a mock Formula base class and common DSL methods
        // to prevent evaluation errors when loading .rb files.
        let setup_script = r#"
            class Formula
              def self.url(val); end
              def self.sha256(val); end
              def self.license(val); end
              def self.homepage(val); end
              def self.desc(val); end
              def self.stable; yield if block_given?; end
              def self.bottle; end
              def self.head; end
              def self.depends_on(val); end
              def self.resource(name); end
              def self.patch; end

              # Instance methods available during 'install'
              def system(*args)
                # Map to Rust implementation
                Rust.system(args)
              end

              def bin
                # Return an object that responds to 'install'
                Installer.new("bin")
              end

              def prefix
                Installer.new("prefix")
              end
            end

            class Installer
              def initialize(type)
                @type = type
              end

              def install(*args)
                # Map to Rust implementation
                Rust.install(@type, args)
              end
            end
        "#;

        self.ruby.eval::<magnus::Value>(setup_script)
            .map_err(|e| BruError::Other(anyhow::anyhow!("Ruby setup error: {}", e)))?;
        
        // Define the Rust bridge module
        let rust_module = self.ruby.define_module("Rust")
            .map_err(|e| BruError::Other(anyhow::anyhow!("Failed to define Rust module: {}", e)))?;

        rust_module.define_module_function("system", magnus::function!(rust_system, 1))
            .map_err(|e| BruError::Other(anyhow::anyhow!("Failed to define Rust.system: {}", e)))?;

        rust_module.define_module_function("install", magnus::function!(rust_install, 2))
            .map_err(|e| BruError::Other(anyhow::anyhow!("Failed to define Rust.install: {}", e)))?;

        Ok(())
    }

    /// Load a formula from a .rb file and return the class object
    pub fn load_formula(&self, path: &Path, name: &str) -> Result<magnus::RClass> {
        if !path.exists() {
            return Err(BruError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Formula file not found: {}", path.display()),
            )));
        }

        let content = std::fs::read_to_string(path)?;
        
        // Evaluate the formula file
        self.ruby.eval::<magnus::Value>(&content)
            .map_err(|e| BruError::Other(anyhow::anyhow!("Ruby evaluation error: {}", e)))?;

        // Constantize the class name (e.g., "ripgrep" -> "Ripgrep")
        let class_name = self.ruby.eval::<String>(&format!(
            "Object.constants.find {{ |c| c.to_s.downcase == '{}' }}",
            name.replace('-', "")
        )).map_err(|e| BruError::Other(anyhow::anyhow!("Class lookup error: {}", e)))?;

        self.ruby.eval::<magnus::RClass>(&class_name)
            .map_err(|e| BruError::Other(anyhow::anyhow!("Failed to get class {}: {}", class_name, e)))
    }

    /// Execute the 'install' method of a formula class
    pub fn execute_install(&self, class: magnus::RClass) -> Result<()> {
        // Create an instance of the formula
        let instance = class.new_instance(())
            .map_err(|e| BruError::Other(anyhow::anyhow!("Failed to instantiate formula: {}", e)))?;

        // Call the 'install' method
        let _: magnus::Value = instance.funcall("install", ())
            .map_err(|e| BruError::Other(anyhow::anyhow!("Ruby install method failed: {}", e)))?;

        Ok(())
    }
}

/// Rust implementation of Ruby 'system' method
fn rust_system(args: Vec<String>) -> bool {
    if args.is_empty() {
        return false;
    }

    println!("  {} {}", "system".blue().bold(), args.join(" ").dimmed());

    let status = std::process::Command::new(&args[0])
        .args(&args[1..])
        .status();

    match status {
        Ok(s) => s.success(),
        Err(e) => {
            eprintln!("  {}: {}", "Error".red().bold(), e);
            false
        }
    }
}

/// Rust implementation of Homebrew 'install' DSL (e.g., bin.install)
fn rust_install(install_type: String, files: Vec<String>) {
    println!("  {} {} files to {}", "install".green().bold(), files.len(), install_type.cyan());
    
    for file in files {
        println!("    ├ {}", file.dimmed());
        // For MVP, we just log. Real implementation will move files to Cellar.
    }
}

/// Verification function for Ruby integration
pub fn test_ruby(ruby: &Ruby) -> Result<()> {
    let loader = FormulaLoader::new(ruby);
    loader.setup_environment()?;

    println!("Ruby environment initialized. Testing evaluation...");

    let script = r#"
        class TestFormula < Formula
          url "https://example.com/test-1.0.tar.gz"
          sha256 "deadbeef"
          
          def install
            puts "Hello from Ruby install method!"
            system "echo", "System call from Ruby works!"
            bin.install "my-binary"
          end
        end
    "#;

    // Load the formula (evaluate the class definition)
    ruby.eval::<magnus::Value>(script)
        .map_err(|e| BruError::Other(anyhow::anyhow!("Class definition failed: {}", e)))?;

    // Get the class and execute install
    let class = ruby.eval::<magnus::RClass>("TestFormula")
        .map_err(|e| BruError::Other(anyhow::anyhow!("Class lookup failed: {}", e)))?;

    loader.execute_install(class)?;

    println!("Ruby test result: Success");
    Ok(())
}