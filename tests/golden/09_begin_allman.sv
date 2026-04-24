// config: begin_style=allman
module m;
initial begin
x = 1;
end
endmodule
// expected -----
module m;
  initial
  begin
    x = 1;
  end
endmodule
