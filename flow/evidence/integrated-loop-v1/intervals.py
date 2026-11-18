def merge_intervals(intervals, *, join_touching=False):
    if not isinstance(join_touching, bool):
        raise ValueError("join_touching must be a boolean")
    if not isinstance(intervals, (list, tuple)):
        raise ValueError("intervals must be a list or tuple of interval pairs")

    validated = []
    for interval in intervals:
        if not isinstance(interval, (list, tuple)) or len(interval) != 2:
            raise ValueError("each interval must be a pair")
        start, end = interval
        if (
            not isinstance(start, int)
            or isinstance(start, bool)
            or not isinstance(end, int)
            or isinstance(end, bool)
            or start >= end
        ):
            raise ValueError("endpoints must be integers with start < end")
        validated.append((start, end))

    result = []
    for start, end in sorted(validated):
        if result and (
            start < result[-1][1] or (join_touching and start == result[-1][1])
        ):
            result[-1] = (result[-1][0], max(end, result[-1][1]))
        else:
            result.append((start, end))
    return result
