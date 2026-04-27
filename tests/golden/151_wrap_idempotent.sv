module m;
  initial begin
    if (
      a &&
      b &&
      c
    ) begin
      x = 1;
    end
  end
endmodule
// expected -----
module m;
  initial begin
    if (
      a &&
      b &&
      c
    ) begin
      x = 1;
    end
  end
endmodule
