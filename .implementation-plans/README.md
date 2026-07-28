# Implementation Plans

This directory stores implementation plans generated for staged agent execution.

`queue-plans.yaml` is the source of truth for plan status, order, dependencies, and execution prompts. Plans live under `plans/`; every plan is a directory containing round files and an inner `queue-rounds.yaml`.

Execute one round at a time. Status lives in YAML; files and directories do not move between states.
