# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import setuptools
import sys

try:
    import branp
except ImportError:
    print("Error importing branp modules.")
    sys.exit(1)

long_description = open("README.md", "r", encoding="utf-8").read()


def main() -> None:
    setuptools.setup(
        name="branp",
        version=branp.__version__,
        author="Brandon Pacewic",
        description="Collection of personal CLI tools.",
        long_description_content_type="text/markdown",
        long_description=long_description,
        license="MIT",
        classifiers=[
            "Programming Language :: Python :: 3",
            "License :: OSI Approved :: MIT License"
        ],
        url="https://github.com/BrandonPacewic/branp",
        packages=["branp"],
        entry_points={
            "console_scripts": [
                "branp=branp.__main__:main"
            ]
        },
        python_requires=">=3.11",
        include_package_data=True,
    )


if __name__ == "__main__":
    main()
