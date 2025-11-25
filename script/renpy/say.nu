#!/usr/bin/env nu

def main [s: string] {
    $s | http post 'http://127.0.0.1:17889/submit'
}
