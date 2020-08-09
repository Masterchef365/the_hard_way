#!/bin/sh

compile() {
    glslc -g -O $1 -o $1.spv
}

compile triangle.vert
compile triangle.frag
