import yaml

mapping = {
    'dei': 'orchestrator',
    'ars': 'skills',
    'fabrica': 'forge',
    'codex': 'database',
    'clavis': 'secrets',
    'oratio': 'speech',
    'populi': 'ml',
    'ludus': 'gamification',
    'schola': 'tutorial',
    'pm': 'package_manager',
    'arca': 'package_manager'
}

with open("contracts/operations/catalog.v1.yaml", "r") as f:
    data = yaml.safe_load(f)

for op in data.get("operations", []):
    op_id = op.get("id", "")
    if op_id in mapping:
        op["canonical_name"] = mapping[op_id]
        op["latin_aliases"] = [op_id]
    else:
        op["canonical_name"] = op_id
        op["latin_aliases"] = []

with open("contracts/operations/catalog.v1.yaml", "w") as f:
    yaml.dump(data, f, sort_keys=False, default_flow_style=False)
