"""Validate synthetic HTTP request/response captures with OpenAPI JSON Schema.

QA-only dependencies: openapi-schema-validator==0.9.0, jsonschema==4.26.0.
Run inside an isolated venv; no network, server, credentials or writes here.
"""
import json
import sys
import warnings

with warnings.catch_warnings():
    warnings.simplefilter("ignore", DeprecationWarning)
    from jsonschema import RefResolver
from openapi_schema_validator import OAS31Validator


with open("docs/api/openapi-2026-10-14.json", encoding="utf-8") as source:
    spec = json.load(source)
with open(sys.argv[1], encoding="utf-8") as capture:
    samples = json.load(capture)
resolver = RefResolver.from_schema(spec)
count = 0
for sample in samples:
    operation = spec["paths"][sample["route"]][sample["method"]]
    response = operation["responses"][str(sample["status"])]
    if "$ref" in response:
        response = spec["components"]["responses"][response["$ref"].split("/")[-1]]
    schemas = [(sample["response"], response["content"]["application/json"]["schema"])]
    if "request" in sample:
        schemas.append((sample["request"], operation["requestBody"]["content"]["application/json"]["schema"]))
    for payload, schema in schemas:
        OAS31Validator(schema, resolver=resolver).validate(payload)
        count += 1
print(f"OpenAPI 3.1 request/response schemas PASS: {count} payloads, {len(samples)} real HTTPS responses")
