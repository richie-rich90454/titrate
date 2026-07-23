# packaging

The `tt::packaging` module provides utilities for building, installing, and distributing Titrate modules. It includes distutils-like functionality for package management.

## Distutils

The `Distutils` module provides package distribution utilities:

### Distribution

Create and configure a package distribution:

```titrate
import tt::packaging::Distutils;

let dist: Distribution = new Distribution("my-package", "1.0.0");
dist.setAuthor("Your Name");
dist.setDescription("A Titrate package");
```

### Extension

Define C/C++ extensions to compile:

```titrate
let ext: Extension = new Extension("myext", sources);
ext.addIncludeDir("/usr/include");
ext.addLibrary("m");
ext.setLanguage("c");
```

### find_packages

Discover packages in a directory:

```titrate
let packages: ArrayList<string> = Distutils.find_packages("src");
```

## Site

The `Site` module provides site-specific configuration:

```titrate
import tt::packaging::Site;

let sitePackages: string = Site.getSitePackages();
let userSite: string = Site.getUserSitePackages();
```

## Venv

The `Venv` module creates virtual environments:

```titrate
import tt::packaging::Venv;

Venv.create("myenv");
Venv.activate("myenv");
```

## ZipApp

Create executable Python-like zip archives:

```titrate
import tt::packaging::ZipApp;

ZipApp.create("app.pyz", "app_directory", "/usr/bin/env trc");
```

## Module Functions Reference

| Function | Description |
|----------|-------------|
| `Distribution(name, version)` | Create package distribution |
| `Extension(name, sources)` | Define C extension |
| `find_packages(dir)` | Discover packages |
| `Site.getSitePackages()` | Get site-packages path |
| `Venv.create(name)` | Create virtual environment |
| `ZipApp.create(...)` | Create zip application |
