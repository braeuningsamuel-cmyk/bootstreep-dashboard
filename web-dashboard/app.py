"""Smoke-test app for bootstreep web-dashboard CI validation.

This file is a minimal Flask app used by .github/workflows/web-dashboard-ci.yml
to verify Python syntax and Flask wiring. The real dashboard lives at
C:\\Users\\Samuel\\web-dashboard\\backend\\app.py on the homelab host.

This stub exists only so CI can compile-check the structure without
needing the homelab-only dependencies (psutil, GitPython, real homelab path).
"""

from flask import Flask, jsonify

app = Flask(__name__)


@app.route("/")
def index():
    return "bootstreep web-dashboard (CI stub)"


@app.route("/api/health")
def health():
    return jsonify({"status": "ok"})


if __name__ == "__main__":
    app.run(host="127.0.0.1", port=5000)