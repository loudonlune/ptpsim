
from pydantic import BaseModel

class Timespec(BaseModel):
    sec: int
    nsec: int

    def add(self, other: 'Timespec') -> 'Timespec':
        total_sec = self.sec + other.sec
        total_nsec = self.nsec + other.nsec

        if total_nsec >= 1_000_000_000:
            total_sec += 1
            total_nsec -= 1_000_000_000

        return Timespec(sec=total_sec, nsec=total_nsec)
    
    def subtract(self, other: 'Timespec') -> 'Timespec':
        total_sec = self.sec - other.sec
        total_nsec = self.nsec - other.nsec

        if total_nsec < 0:
            total_sec -= 1
            total_nsec += 1_000_000_000

        return Timespec(sec=total_sec, nsec=total_nsec)
    
    def multiply(self, multiplier: int) -> 'Timespec':
        total_nsec = (self.sec * 1_000_000_000 + self.nsec) * multiplier
        return Timespec(sec=int(total_nsec // 1_000_000_000), nsec=total_nsec % 1_000_000_000)

    def divide(self, divisor: int) -> 'Timespec':
        total_nsec = self.sec * 1_000_000_000 + self.nsec
        divided_nsec = total_nsec // divisor
        return Timespec(sec=int(divided_nsec // 1_000_000_000), nsec=divided_nsec % 1_000_000_000)

