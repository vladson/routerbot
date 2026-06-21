# Kubernetes

Some Routerbot capabilities may be backed by Kubernetes workloads.

The first Kubernetes-backed DLNA implementation should use a rollout restart by
patching a Deployment annotation. This is safer and simpler than executing commands
inside pods.

Kubernetes credentials should be least-privilege and scoped to the required namespace
and workload operations.
